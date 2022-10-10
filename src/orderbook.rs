#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use barter_integration::model::Side;
use std::cmp::Ordering;
use crate::model::{AtomicOrder, Order, OrderBookEvent};

/// Todo: NonNan wrapper for ordering floats in Vec
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
pub struct OrderBookL3 {
    pub last_sequence: u64,
    pub bids: Vec<OrderDeque>,
    pub asks: Vec<OrderDeque>,
    /// todo: consider replacing f64 with Arc<Mutex<OrderDeque>>
    pub order_id_map: HashMap<String, (Side, NonNan)>,
}

/// todo: refactor insert/remove/update to reuse code
impl OrderBookL3 {
    pub fn new() -> Self {
        OrderBookL3 {
            last_sequence: 0,
            bids: Vec::new(),
            asks: Vec::new(),
            order_id_map: HashMap::new(),
        }
    }

    pub fn process(&mut self, event: OrderBookEvent) {
        match event {
            OrderBookEvent::Received(_, _) => {
                // todo: make use of these
                // updating the orderbook based on received messages generally
                // only makes sense if we can model the orderbook changes faster than the
                // websocket can give us the resulting updates in ensuing messages.
            }
            OrderBookEvent::Open(order, sequence) => {
                if sequence > self.last_sequence {
                    self.insert(order);
                }
            }
            OrderBookEvent::Done(order_id, sequence) => {
                if sequence > self.last_sequence {
                    self.remove(order_id);
                }
            }
            OrderBookEvent::Change(order_id, new_size, sequence) => {
                if sequence > self.last_sequence {
                    self.update(order_id, new_size);
                }
            }
        }
    }

    /// Find order deque and push order to the back. If order deque does not exist,
    /// initialize one with order included and insert into the orderbook.
    ///
    /// Also inserts {order_id: (Side, Price)} pair into map to assist in order retrieval.
    ///
    /// Skips insertion of orders with Nan floats.
    fn insert(&mut self, order: Order) {
        match order {
            Order::Bid(order, _) => {
                let maybe_num = NonNan::build(order.price.clone());
                if let Some(price) = maybe_num {
                    self.order_id_map.insert(order.id.clone(), (Side::Buy, price.clone()));
                    match self.bids.binary_search_by_key(&price, |order_deque| order_deque.price) {
                        Ok(pos) => { self.bids[pos].push_back(order) }
                        Err(pos) => { self.bids.insert(pos, OrderDeque::build(order)) }
                    }
                }
            },
            Order::Ask(order, _) => {
                let maybe_num = NonNan::build(order.price.clone());
                if let Some(price) = maybe_num {
                    self.order_id_map.insert(order.id.clone(), (Side::Sell, price.clone()));
                    match self.asks.binary_search_by_key(&price, |order_deque| order_deque.price) {
                        Ok(pos) => { self.asks[pos].push_back(order) }
                        Err(pos) => { self.asks.insert(pos, OrderDeque::build(order)) }
                    }
                }
            }
        }
    }

    /// todo: add error handling for missing order deques
    ///
    /// Gets price and side associated with order_id, finds the order deque, and calls its remove method.
    ///
    /// If order deque is left with no orders, remove it.
    ///
    /// Also remove order_id key from map.
    fn remove(&mut self, order_id: String) {
        if let Some((side, price)) = self.order_id_map.get(&order_id) {
            match side {
                Side::Buy => {
                    match self.bids.binary_search_by_key(price, | order_deque| order_deque.price) {
                        Ok(pos) => {
                            self.bids[pos].remove(&order_id);
                            if self.bids[pos].len() == 0 {
                                self.bids.remove(pos);
                            }
                        },
                        Err(_) => {},
                    }
                }
                Side::Sell => {
                    match self.asks.binary_search_by_key(price, |order_deque| order_deque.price) {
                        Ok(pos) => {
                            self.asks[pos].remove(&order_id);
                            if self.asks[pos].len() == 0 {
                                self.asks.remove(pos);
                            }
                        },
                        Err(_) => {},
                    }
                }
            }
        }
        self.order_id_map.remove(&order_id);
    }


    /// todo: add error handling for missing order deques
    ///
    /// Gets price and side associated with order_id, finds the order deque, and calls its update method.
    fn update(&mut self, order_id: String, new_size: f64) {
        if let Some((side, price)) = self.order_id_map.get(&order_id) {
            match side {
                Side::Buy => {
                    match self.bids.binary_search_by_key(price, | order_deque| order_deque.price) {
                        Ok(pos) => {self.bids[pos].update(&order_id, new_size)},
                        Err(_) => {},
                    }
                }
                Side::Sell => {
                    match self.asks.binary_search_by_key(price, | order_deque| order_deque.price) {
                        Ok(pos) => {self.asks[pos].update(&order_id, new_size)},
                        Err(_) => {},
                    }
                }
            }
        }
    }

    pub fn num_bid_levels(&self) -> usize {
        self.bids.len()
    }

    pub fn num_ask_levels(&self) -> usize {
        self.asks.len()
    }

    pub fn best_bid(&self) -> f64 {
        self.bids[0].price.0
    }

    pub fn best_ask(&self) -> f64 {
        self.bids[0].price.0
    }

    /// Return vector of (f64, f64, f64) tuples representing current snapshot of price, marginal
    /// order size (aggregate order size at each level)
    /// and cumulative depth (integral of price * order size)
    fn levels(&self, side: Side) -> Vec<(f64, f64, f64)> {
        match side {
            Side::Buy => {
                self.bids.iter().scan(0.0, |cumsum, deque| Option::from({
                    *cumsum += deque.price.0 * deque.size();
                    (deque.price.0, deque.size(), cumsum.clone())
                })).collect()
            },
            Side::Sell => {
                self.asks.iter().scan(0.0, |cumsum, deque| Option::from({
                    *cumsum += deque.price.0 * deque.size();
                    (deque.price.0, deque.size(), cumsum.clone())
                })).collect()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn orderbook_l3_basics() {

    }

}