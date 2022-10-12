#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
// standard
use std::collections::{HashMap, VecDeque, HashSet};
use std::cmp::{Ordering, Reverse};
use std::fmt::{Display, Formatter};
// external
use barter_integration::model::{Exchange, Instrument, Market, Side};
use thiserror::Error;
// internal
use crate::model::{AtomicOrder, Order, OrderbookEvent, Sequence, OrderId};

/// Collection of ['OrderBookL3'] structs.
#[derive(Debug)]
pub struct OrderbookMap {
    pub map: HashMap<Market, OrderbookL3>
}

impl OrderbookMap {
    pub fn new() -> Self {
        OrderbookMap {
            map: HashMap::new()
        }
    }

    pub fn get(&self, market: &Market) -> Option<&OrderbookL3>{
        self.map.get(market)
    }

    pub fn get_mut(&mut self, market: &Market) -> Option<&mut OrderbookL3> {
        self.map.get_mut(market)
    }
}


/// NonNan wrapper for ordering floats in Vec, for orderbook sortability.
/// Todo:
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct NonNan(f64);

impl NonNan {
    fn build(val: f64) -> Option<Self> {
        if val.is_nan() { None }
        else { Some(NonNan(val)) }
    }
}

impl Eq for NonNan {}

impl Ord for NonNan {
    fn cmp(&self, other: &NonNan) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

/// Todo
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct OrderDeque {
    pub deque: VecDeque<AtomicOrder>,
    pub price: NonNan,
}

impl OrderDeque {
    fn build(order: AtomicOrder) -> Self {
        let mut deque = OrderDeque {
            deque: VecDeque::new(),
            price: NonNan::build(order.price.clone()).unwrap(),
        };
        deque.push_back(order);
        deque
    }

    fn push_back(&mut self, order: AtomicOrder) {
        self.deque.push_back(order)
    }

    fn remove(&mut self, order_id: &String) {
        self.deque.retain(|order| order.id != *order_id)
    }

    fn update(&mut self, order_id: &String, new_size: f64) {
        let idx = self.deque
            .iter_mut()
            .position(|order| order.id == *order_id);

        if idx.is_some() {
            self.deque[idx.unwrap()].size = new_size;
        }
    }

    fn size(&self) -> f64 {
        self.deque.iter().fold(0.0, |a, b| a + b.size)
    }

    fn len(&self) -> usize {
        self.deque.len()
    }
}

/// Todo: consider using something other than a Vec for bids/asks
#[derive(Clone, Debug)]
pub struct OrderbookL3 {
    // info
    pub market: Market,
    pub last_sequence: u64,

    // data structures
    pub bids: Vec<OrderDeque>,
    pub asks: Vec<OrderDeque>,
    // todo: consider replacing (Side, NonNan) with raw pointer or Arc Mutex to OrderDeque
    pub order_id_map: HashMap<String, (Side, NonNan)>,

    // factor-based outlier identification
    pub ignore_outliers: bool,
    pub outlier_factor: f64,
    pub bid_cutoff: f64,
    pub ask_cutoff: f64,
    pub outlier_ids: HashSet<OrderId>,

    // stats
    pub events_processed: u64,
    pub events_not_processed: u64,

    // errors
    pub track_errors: bool,
    pub error_msgs: Option<HashSet<String>>,
}

/// todo: refactor insert/remove/update to reuse code
impl OrderbookL3 {
    pub fn build(exchange: Exchange, instrument: Instrument) -> Self {
        OrderbookL3 {
            market: Market::new(exchange, instrument),
            last_sequence: 0,
            bids: Vec::new(),
            asks: Vec::new(),
            order_id_map: HashMap::new(),
            ignore_outliers: false,
            outlier_factor: 2.0,
            bid_cutoff: 0.0,
            ask_cutoff: 0.0,
            outlier_ids: HashSet::new(),
            events_processed: 0,
            events_not_processed: 0,
            track_errors: true,
            error_msgs: Some(HashSet::new()),
        }
    }

    pub fn process(&mut self, event: OrderbookEvent) {
        let sequence = event.sequence();
        let result: Result<(), OrderbookError> = match &sequence.cmp(&self.last_sequence) {
            Ordering::Greater => {
                match &event {
                    // todo: received orders do not change state of orderbook
                    // todo: make use of these to model orderbook changes faster than websocket confirms them?
                    OrderbookEvent::Received(_order, _) => Ok(()),
                    OrderbookEvent::Open(order, _) => self.insert(order),
                    OrderbookEvent::Done(order_id, _) => self.remove(order_id),
                    OrderbookEvent::Change(order_id, new_size, _) => self.update(order_id, new_size),
                }
            },
            _ => Err(OrderbookError::OutOfSequence(event))
        };
        self.update_sequence_and_stats(result, sequence);
    }

    fn update_sequence_and_stats(&mut self, result: Result<(), OrderbookError>, sequence: Sequence) {
        match result {
            Ok(()) => {
                self.last_sequence = sequence.clone();
                self.events_processed += 1;
            },
            Err(OrderbookError::OutOfSequence(event)) => {
                self.events_not_processed += 1;
                self.error_msgs
                    .as_mut()
                    .map(|map| map.insert(format!("{:?}", OrderbookError::OutOfSequence(event))));
            },
            Err(error) => {
                self.last_sequence = sequence.clone();
                self.events_not_processed += 1;
                self.error_msgs
                    .as_mut()
                    .map(|map| map.insert(format!("{:?}", error)));
            },
        }
    }

    /// Find order deque and push order to the back. If order deque does not exist,
    /// initialize one with order included and insert into the orderbook.
    ///
    /// Also inserts {order_id: (Side, Price)} pair into map to assist in order retrieval.
    ///
    /// Skips insertion of orders with Nan floats.
    fn insert(&mut self, order: &Order) -> Result<(), OrderbookError> {
        match order {
            Order::Bid(order, _) => {
                let maybe_num = NonNan::build(order.price);
                if let Some(price) = maybe_num {
                    self.order_id_map.insert(order.id.clone(), (Side::Buy, price.clone()));
                    match self.bids.binary_search_by_key(&price, |order_deque| order_deque.price) {
                        Ok(pos) => { self.bids[pos].push_back(order.clone()) }
                        Err(pos) => { self.bids.insert(pos, OrderDeque::build(order.clone())) }
                    };
                    Ok(())
                } else {
                    Err(OrderbookError::NanFloat(order.clone()))
                }
            },
            Order::Ask(order, _) => {
                let maybe_num = NonNan::build(order.price.clone());
                if let Some(price) = maybe_num {
                    self.order_id_map.insert(order.id.clone(), (Side::Sell, price.clone()));
                    match self.asks.binary_search_by_key(&price, |order_deque| order_deque.price) {
                        Ok(pos) => { self.asks[pos].push_back(order.clone()) }
                        Err(pos) => { self.asks.insert(pos, OrderDeque::build(order.clone())) }
                    };
                    Ok(())
                } else {
                    Err(OrderbookError::NanFloat(order.clone()))
                }
            }
        }
    }

    /// Gets price and side associated with order_id, finds the order deque, and calls its remove method.
    ///
    /// If order deque is left with no orders, remove it.
    ///
    /// Also remove order_id key from map.
    fn remove(&mut self, order_id: &String) -> Result<(), OrderbookError> {
        if let Some((side, price)) = self.order_id_map.get(&*order_id) {
            match side {
                Side::Buy => {
                    match self.bids.binary_search_by_key(price, | order_deque| order_deque.price) {
                        Ok(pos) => {
                            self.bids[pos].remove(&order_id);
                            if self.bids[pos].len() == 0 {
                                self.bids.remove(pos);
                            };
                            Ok(())
                        },
                        Err(_) => Err(OrderbookError::MissingOrderDeque(price.clone())),
                    }
                }
                Side::Sell => {
                    match self.asks.binary_search_by_key(price, |order_deque| order_deque.price) {
                        Ok(pos) => {
                            self.asks[pos].remove(&order_id);
                            if self.asks[pos].len() == 0 {
                                self.asks.remove(pos);
                            };
                            Ok(())
                        },
                        Err(_) => Err(OrderbookError::MissingOrderDeque(price.clone())),
                    }
                }
            }
        } else {
            Err(OrderbookError::OrderNotFoundInMap(order_id.clone()))
        }
    }


    /// todo: add error handling for missing order deques
    ///
    /// Gets price and side associated with order_id, finds the order deque, and calls its update method.
    fn update(&mut self, order_id: &String, new_size: &f64) -> Result<(), OrderbookError> {
        if let Some((side, price)) = self.order_id_map.get(&*order_id) {
            match side {
                Side::Buy => {
                    match self.bids.binary_search_by_key(price, | order_deque| order_deque.price) {
                        Ok(pos) => {Ok(self.bids[pos].update(order_id, new_size.clone()))},
                        Err(_) => Err(OrderbookError::MissingOrderDeque(price.clone())),
                    }
                }
                Side::Sell => {
                    match self.asks.binary_search_by_key(price, | order_deque| order_deque.price) {
                        Ok(pos) => {Ok(self.asks[pos].update(order_id, new_size.clone()))},
                        Err(_) => Err(OrderbookError::MissingOrderDeque(price.clone())),
                    }
                }
            }
        } else {
            Err(OrderbookError::OrderNotFoundInMap(order_id.clone()))
        }
    }

    pub fn num_bid_levels(&self) -> usize {
        self.bids.len()
    }

    pub fn num_ask_levels(&self) -> usize {
        self.asks.len()
    }

    pub fn best_bid(&self) -> Option<f64> {
        self.bids.iter().map(|orders| orders.price.0).take(1).next()
    }

    pub fn best_ask(&self) -> Option<f64> {
        self.asks.iter().map(|orders| orders.price.0).take(1).next()
    }

    pub fn get(&self, order_id: String) -> Option<&AtomicOrder> {
        todo!()
    }

    /// Return vector of (f64, f64, f64) tuples representing current snapshot of price, marginal
    /// order size (aggregate order size at each level)
    /// and running of volume/liquidity (integral of price * order size)
    fn levels(&self, side: Side, depth: Option<usize>) -> Vec<(f64, f64, f64)> {
        match side {
            Side::Buy => {
                let scan = self.bids.iter().scan(0.0, |cumsum, deque| Option::from({
                    *cumsum += deque.price.0 * deque.size();
                    (deque.price.0, deque.size(), cumsum.clone())
                }));
                match depth {
                    Some(n) => scan.take(n).collect(),
                    None => scan.collect()
                }

            },
            Side::Sell => {
                let scan = self.asks.iter().scan(0.0, |cumsum, deque| Option::from({
                    *cumsum += deque.price.0 * deque.size();
                    (deque.price.0, deque.size(), cumsum.clone())
                }));
                match depth {
                    Some(n) => scan.take(n).collect(),
                    None => scan.collect()
                }
            },
        }
    }
}

#[derive(Error, Debug)]
pub enum OrderbookError {
    OutOfSequence(OrderbookEvent),
    OrderNotFoundInMap(OrderId),
    MissingOrderDeque(NonNan),
    NanFloat(AtomicOrder),
}

impl Display for OrderbookError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", &self)
    }
}

#[cfg(test)]
mod tests {
    use barter_integration::model::InstrumentKind;
    use crate::ExchangeId;
    use crate::model::{DataKind, DataKind::OrderBookEvent, OrderbookEvent, OrderType};
    use crate::model::Order;
    use crate::model::OrderbookEvent::{Done, Open, Change};
    use super::*;
    use rand::Rng;

    fn gen_rand_ob_events_simple(num: usize) -> Vec<OrderbookEvent> {
        let mut events: Vec<OrderbookEvent> = Vec::new();
        let mut rng = rand::thread_rng();
        let mid_price: f64 = rng.gen_range(1000..2000) as f64 / 10.0;

        for _ in 0..num {

            match rng.gen_bool(0.50) {
                // bids
                true => {
                    let price: f64 = rng.gen_range(mid_price..mid_price * 1.5);
                    let size: f64 = rng.gen_range(0..100) as f64 / 10.0;
                },
                // asks
                false => {
                    let price: f64 = rng.gen_range(mid_price..mid_price / 1.5);
                    let size: f64 = rng.gen_range(0..100) as f64 / 10.0;

                }
            }
        }
        events
    }

    #[test]
    pub fn orderbook_l3_fuzzing() {
        let instrument = Instrument::from(("eth", "usd", InstrumentKind::Spot));
        let exchange = Exchange::from(ExchangeId::Coinbase);
        let orderbook = OrderbookL3::build(exchange, instrument);

        todo!()

    }

    #[test]
    pub fn orderbook_l3_basics() {
        let instrument = Instrument::from(("eth", "usd", InstrumentKind::Spot));
        let exchange = Exchange::from(ExchangeId::Coinbase);
        let mut orderbook = OrderbookL3::build(exchange, instrument);

        let invalid_events: Vec<OrderbookEvent> = vec![
            Done("H".to_string() , 18),
            Change("G".to_string(), 30.0, 14),
            Done("F".to_string() , 17),
            Done("ZZ".to_string() , 100),
        ];

        invalid_events.into_iter().for_each(|event| orderbook.process(event));

        // test empty book
        assert_eq!(orderbook.bids, vec![]);
        assert_eq!(orderbook.asks, vec![]);
        assert_eq!(orderbook.best_ask(), None);
        assert_eq!(orderbook.best_bid(), None);
        assert_eq!(orderbook.levels(Side::Buy, None), vec![]);
        assert_eq!(orderbook.levels(Side::Buy, None), vec![]);
        assert_eq!(orderbook.num_ask_levels(), 0);
        assert_eq!(orderbook.num_bid_levels(), 0);


        // 3 ask levels, 4 bid levels post-insert
        let open_events= vec![
            Open(Order::Ask(AtomicOrder { id: "A".to_string(), price: 1005.0, size: 20.0 }, OrderType::Limit), 1),
            Open(Order::Bid(AtomicOrder { id: "B".to_string(), price: 995.0, size: 5.0 }, OrderType::Limit), 2),
            Open(Order::Ask(AtomicOrder { id: "C".to_string(), price: 1006.0, size: 1.0 }, OrderType::Limit), 3),
            Open(Order::Bid(AtomicOrder { id: "D".to_string(), price: 994.0, size: 2.0 }, OrderType::Limit), 4),
            Open(Order::Ask(AtomicOrder { id: "E".to_string(), price: 1005.0, size: 0.25 }, OrderType::Limit), 5),
            Open(Order::Bid(AtomicOrder { id: "F".to_string(), price: 997.0, size: 10.0 }, OrderType::Limit), 6),
            Open(Order::Ask(AtomicOrder { id: "G".to_string(), price: 1001.0, size: 4.0 }, OrderType::Limit), 7),
            Open(Order::Bid(AtomicOrder { id: "H".to_string(), price: 996.0, size: 3.0 }, OrderType::Limit), 8),
            Open(Order::Ask(AtomicOrder { id: "I".to_string(), price: 1005.0, size: 10.0 }, OrderType::Limit), 9),
            Open(Order::Bid(AtomicOrder { id: "J".to_string(), price: 994.0, size: 6.0 }, OrderType::Limit), 10),
        ];

        open_events.into_iter().for_each(|event| orderbook.process(event));

        println!("{:?}", orderbook.levels(Side::Buy, None));
        println!("{:?}", orderbook.levels(Side::Sell, None));
        assert_eq!(orderbook.best_bid(), Some(997.0));
        assert_eq!(orderbook.best_ask(), Some(1001.0));

        let change_events = vec![
            Change("A".to_string(), 30.0, 11),
            Change("B".to_string(), 30.0, 12),
            Change("C".to_string(), 30.0, 13),
            Change("D".to_string(), 30.0, 14),
        ];

        // 2 ask levels, 2 bid levels post-removal
        let close_events = vec![
            Done("E".to_string() , 15),
            Done("F".to_string() , 16),
            Done("G".to_string() , 17),
            Done("H".to_string() , 18),
        ];

        // invalid events (out-of-sequence or missing)
        let invalid_events = vec![
            Done("J".to_string() , 18),
            Open(Order::Bid(AtomicOrder { id: "D".to_string(), price: 994.0, size: 2.0 }, OrderType::Limit), 4),
            Change("G".to_string(), 30.0, 14),
            Done("ZZ".to_string(), 19),
        ];

        assert_eq!(orderbook.best_bid(), Some(995.0));
        assert_eq!(orderbook.best_ask(), Some(1005.0));
        assert_eq!(orderbook.num_ask_levels(), 2);
        assert_eq!(orderbook.num_bid_levels(), 2);
        println!("{:?}", orderbook.num_bid_levels());
        println!("{:?}", orderbook.bids);
        println!("{:?}", orderbook.asks);
        println!("{:?}", orderbook.levels(Side::Buy, None));
        println!("{:?}", orderbook.levels(Side::Sell, None));

    }

}