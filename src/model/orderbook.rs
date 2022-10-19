//! # Features
//! - No unsafe rust used
//! - Vector-based bid and ask books with a VecDeque for each price level
//! - Iteration over entire book, insert/remove/update, liquidity curve (see levels method),
//! order refs/muts by id, and more
//! - Crude sequence-checking: messages with a stale sequence are ignored
//! - Optional simple factor-based outlier filter. choose a % deviation bound from best bid/ask and
//! orderbook will skip processing orders with prices outside the bound. This is especially useful for
//! exchanges which constantly broadcast extreme limit orders that are unlikely to ever fill - these
//! orders only serve to slow down vector-based books.
//!
//! # Time Complexity
//! - order deque insertion: O(logN+N) - find insert position via binary search and insert into vector
//! - order insertion: O(logN+M) - find deque via binary search and then push order to the back
//! - order removal: O(logN+M) - find deque via binary search and then remove order
//!     - if deque is left empty, removal is an additional O(N) operation
//! - order retrieval/update: O(logN+M) - find deque via binary search, then find order via linear search
//!
//! # Todos
//! - Consider ways to reduce time complexity of orderbook operations while retaining cache-friendliness.
//! - Improve sequence-checking: add new error for missing sequences.
//! - Implement snapshot loading and sync mechanism (for snapshots retrieved through REST).
//! This is dependent on improved sequence-checking.
//! - Simple stats tracking - generalize this and add more stats to track.
//! - Add option to swap in other data structures as desired.

// standard
use std::collections::{HashMap, HashSet, VecDeque};
use std::cmp::{Ordering, Reverse};
use std::fmt::{Display, Formatter};
use std::iter::{Peekable, Rev};
// external
use barter_integration::model::{Market, Side};
use chrono::{DateTime, Duration, Utc};
use thiserror::Error;
use serde::{Deserialize, Serialize};
use tracing::{warn};
// internal
use crate::model::subscription::de_floats;
// testing
use bounded_vec_deque::BoundedVecDeque;

const DEFAULT_OUTLIER_FACTOR: f64 = 0.50;
const DEFAULT_BEST_BID: f64 = 0.0;
const DEFAULT_BEST_ASK: f64 = 0.0;

pub type NewSize = f64;
pub type Sequence = u64;
pub type OrderDequePos<'a> = (Side, usize, Result<&'a OrderDeque, OrderBookError>);
pub type OrderDequePosMut<'a> = (Side, usize, Result<&'a mut OrderDeque, OrderBookError>);
pub type TopLevel = (f64, f64);

/// Collection of ['OrderBookL3'] structs.
#[derive(Debug)]
pub struct OrderbookMap {
    pub map: HashMap<Market, OrderbookL3>
}

impl OrderbookMap {
    pub fn new() -> Self {
        OrderbookMap { map: HashMap::new() }
    }

    /// Insert orderbook into the orderbook map
    pub fn insert(&mut self, orderbook: OrderbookL3) {
        self.map.insert(orderbook.market.clone(), orderbook);
    }

    pub fn get(&self, market: &Market) -> Option<&OrderbookL3>{
        self.map.get(market)
    }

    pub fn get_mut(&mut self, market: &Market) -> Option<&mut OrderbookL3> {
        self.map.get_mut(market)
    }
}

/// Represents the types of orderbook events
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub enum OrderBookEvent {
    Received(Order, Sequence),
    Open(LimitOrder, Sequence),
    Done(String, Sequence),
    Change(String, NewSize, Sequence),
}

impl OrderBookEvent {
    pub fn sequence(&self) -> Sequence {
        match self {
            OrderBookEvent::Received(_, seq) => seq.clone(),
            OrderBookEvent::Open(_, seq) => seq.clone(),
            OrderBookEvent::Done(_, seq) => seq.clone(),
            OrderBookEvent::Change(_, _, seq) => seq.clone(),
        }
    }
}

/// Enum representing an order variant
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub enum Order {
    MarketOrder(MarketOrder),
    LimitOrder(LimitOrder),
}

/// Market order enum representing the different kinds of market orders
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum MarketOrder {
    BuyWithFunds(f64),
    BuySize(f64),
    SellForFunds(f64),
    SellSize(f64),
}

/// Limit order enum building off of the AtomicOrder struct.
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub enum LimitOrder {
    Bid(AtomicOrder),
    Ask(AtomicOrder),
}

impl LimitOrder {
    pub fn id(&self) -> &str {
        match self {
            LimitOrder::Bid(order, ..) => &order.id,
            LimitOrder::Ask(order, ..) => &order.id,
        }
    }

    pub fn price(&self) -> &f64 {
        match self {
            LimitOrder::Bid(order, ..) => &order.price,
            LimitOrder::Ask(order, ..) => &order.price,
        }
    }

    pub fn side(&self) -> Side {
        match self {
            LimitOrder::Bid(..) => Side::Buy,
            LimitOrder::Ask(..) => Side::Sell,
        }
    }

    pub fn unwrap(&self) -> &AtomicOrder {
        match self {
            LimitOrder::Bid(order, ..) => &order,
            LimitOrder::Ask(order, ..) => &order,
        }
    }
}

/// Simplest order building block
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct AtomicOrder {
    pub id: String,
    #[serde(deserialize_with = "de_floats")]
    pub price: f64,
    #[serde(deserialize_with = "de_floats")]
    pub size: f64,
}

/// Float wrapper with Ord and Eq implementations, for sortability
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

impl TryFrom<f64> for NonNan {
    type Error = OrderBookError;

    /// Attempt to build a NonNan from an f64. Returns OrderbookError::NanFloat(f64) if
    /// the f64 is not a number.
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        NonNan::build(value).ok_or_else(|| OrderBookError::NanFloat(value))
    }
}

/// Double-ended queue of orders
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct OrderDeque {
    pub deque: VecDeque<AtomicOrder>,
    pub price: NonNan,
}

impl OrderDeque {
    /// instantiate new deque with order inserted
    fn build(order: AtomicOrder) -> Self {
        let mut deque = OrderDeque {
            deque: VecDeque::new(),
            price: NonNan::build(order.price.clone()).unwrap(),
        };
        deque.push_back(order);
        deque
    }

    /// push order to the back of the deque
    fn push_back(&mut self, order: AtomicOrder) {
        self.deque.push_back(order)
    }

    /// remove an order by its id
    fn remove(&mut self, order_id: &str) -> Option<AtomicOrder> {
        self.deque.remove(self.get_idx(order_id)?)
    }

    /// get an order's index by its order id
    fn get_idx(&self, order_id: &str) -> Option<usize> {
        self.deque.iter().position(|order| order.id == *order_id)
    }

    /// get a reference to an order by its order id
    fn get_ref(&self, order_id: &str) -> Option<&AtomicOrder> {
        let idx = self.deque.iter().position(|order| order.id == *order_id);
        idx.map(|idx| &self.deque[idx])
    }

    /// get a mutable reference to an order by its order id
    fn get_mut(&mut self, order_id: &str) -> Option<&mut AtomicOrder> {
        let idx = self.deque.iter().position(|order| order.id == *order_id);
        idx.map(|idx| &mut self.deque[idx])
    }

    /// get aggregate of order sizes in deque
    fn size(&self) -> f64 {
        self.deque.iter().fold(0.0, |a, b| a + b.size)
    }

    /// get length of deque
    fn len(&self) -> usize {
        self.deque.len()
    }
}

/// Simple outlier filter that keeps track of outlier order ids.
///
/// Sets hard cutoffs at levels determined by the outlier_factor and
/// the current best bids and asks in the orderbook.
///
/// For bids, this cutoff is best_bid * (1 - outlier_factor),
/// and for asks this cutoff is best_ask * (1 + outlier_factor).
///
/// Enable by calling .outlier_filter() or .outlier_filter_default() methods on
/// the orderbook builder when instantiating an orderbook.
#[derive(Clone, Debug)]
pub struct SimpleOutlierFilter {
    // todo: put sensible bounds on the outlier_factor?
    // todo: consider separate outlier factors for bids and asks?
    pub outlier_factor: f64,
    pub outlier_ids: HashSet<String>,
    pub outliers_encountered: u64,
}

impl SimpleOutlierFilter {
    /// Instantiate a new outlier filter with optional outlier_factor parameter.
    /// If outlier_factor is not provided, it defaults to const DEFAULT_OUTLIER_FACTOR.
    pub fn new(outlier_factor: Option<f64>) -> Self {
        Self {
            outlier_factor: match outlier_factor {
                    Some(factor) => factor,
                    None => DEFAULT_OUTLIER_FACTOR,
            },
            outlier_ids: HashSet::<String>::new(),
            outliers_encountered: 0,
        }
    }

    /// checks if an incoming order is an outlier based on its price in relation to
    /// the best bid, best ask and the outlier factor.
    ///
    /// For all outliers, add order id to hashset and return OrderbookError::Outlier.
    pub fn check(&mut self, order: &LimitOrder, top_level: TopLevel) -> Result<(), OrderBookError> {
        match order.side() {
            Side::Buy => {
                // initial conditions (empty book)
                if top_level.0 == DEFAULT_BEST_BID {
                    Ok(())
                } else {
                    // calculate new cutoff and compare
                    let bid_cutoff = top_level.0 * (1.0 - self.outlier_factor);
                    match order.price().partial_cmp(&bid_cutoff) {
                        Some(Ordering::Less) => {
                            self.outliers_encountered += 1;
                            self.outlier_ids.insert(order.id().to_owned());
                            Err(OrderBookError::Outlier)
                        },
                        _ => Ok(())
                    }
                }
            }
            Side::Sell => {
                // initial conditions (empty book)
                if top_level.1 == DEFAULT_BEST_ASK {
                    Ok(())
                } else {
                    // calculate new cutoff and compare
                    let ask_cutoff = top_level.1 * (1.0 + self.outlier_factor);
                    match order.price().partial_cmp(&ask_cutoff) {
                        Some(Ordering::Greater) => {
                            self.outliers_encountered += 1;
                            self.outlier_ids.insert(order.id().to_owned());
                            Err(OrderBookError::Outlier)
                        },
                        _ => Ok(())
                    }
                }
            }
        }
    }
}

/// Orderbook stats container that can be enabled by passing .stats(bool) to the
/// orderbook builder when instantiating an orderbook.
///
/// Pass .stats(true) to capture error messages, otherwise pass .stats(false).
#[derive(Clone, Debug)]
pub struct OrderbookStats {
    pub events_processed: u64,
    pub events_not_processed: u64,
    pub error_msgs: Option<Vec<String>>,
}

impl OrderbookStats {
    pub fn new(track_errors: bool) -> Self {
        Self {
            events_processed: 0,
            events_not_processed: 0,
            error_msgs: match track_errors {
                true => Some(Vec::<String>::new()),
                false => None,
            },
        }
    }
}

/// Todo: consider alternative data structures for bids and asks
#[derive(Clone, Debug)]
pub struct OrderbookL3 {
    // info
    pub market: Market,
    pub last_sequence: u64,
    pub start_time: DateTime<Utc>,

    // data structures
    pub bids: Vec<OrderDeque>,
    pub asks: Vec<OrderDeque>,
    // todo: consider including raw pointer or Arc Mutex to OrderDeque in the value
    pub order_id_map: HashMap<String, (Side, NonNan)>,

    // optional features
    pub outlier_filter: Option<SimpleOutlierFilter>,
    pub stats: Option<OrderbookStats>,
    pub last_n_events: Option<BoundedVecDeque<OrderBookEvent>>
}

/// todo: refactor insert/remove/update to reuse code
impl OrderbookL3 {
    /// return a builder that will can instantiate an orderbook
    pub fn builder() -> OrderbookBuilder {
        OrderbookBuilder::new()
    }

    /// returns bid level count
    pub fn num_bid_levels(&self) -> usize {
        self.bids.len()
    }

    /// returns ask level count
    pub fn num_ask_levels(&self) -> usize {
        self.asks.len()
    }

    /// returns order count in book
    pub fn len(&self) -> usize { self.iter().count() }

    /// returns bid count in book
    pub fn bid_count(&self) -> usize {
        self.bids.iter().fold(0,|sum, x| sum + x.len())
    }

    /// returns ask count in book
    pub fn ask_count(&self) -> usize {
        self.asks.iter().fold(0, |sum, x| sum + x.len())
    }

    /// returns best bid in orderbook.
    ///
    /// If orderbook is empty, return const DEFAULT_BEST_BID.
    pub fn best_bid(&self) -> f64 {
        self.bids
            .iter()
            .map(|orders| orders.price.0)
            .take(1)
            .next()
            .unwrap_or_else(|| DEFAULT_BEST_BID)
    }

    /// returns best ask in orderbook.
    ///
    /// If orderbook is empty, return const DEFAULT_BEST_ASK.
    pub fn best_ask(&self) -> f64 {
        self.asks
            .iter()
            .map(|orders| orders.price.0)
            .take(1)
            .next()
            .unwrap_or_else(|| DEFAULT_BEST_ASK)
    }

    /// Returns best bid and ask in 2-tuple
    pub fn top_level(&self) -> TopLevel {
        (self.best_bid(), self.best_ask())
    }

    /// for debugging. Store last n events in a bounded VecDeque.
    /// Enabled by calling last_n_events on the orderbook builder.
    pub fn store_event(&mut self, event: &OrderBookEvent) {
        if self.last_n_events.is_some() {
            self.last_n_events.as_mut().unwrap().push_back(event.clone());
        }
    }

    /// Check an OrderbookEvent's sequence and then process
    pub fn process(&mut self, event: OrderBookEvent) {
        self.store_event(&event);

        let sequence = event.sequence();

        let result: Result<(), OrderBookError> = match &sequence.cmp(&self.last_sequence) {
            Ordering::Greater => {

                if sequence != self.last_sequence + 1 {
                    warn!("Missed sequence {} ({:?})", self.last_sequence + 1, event.clone());
                }

                match &event {
                    // todo: received orders do not change state of orderbook but may be used to model
                    // market order impacts before ensuing order limit close messages arrive
                    OrderBookEvent::Received(_order, _) => Ok(()),
                    OrderBookEvent::Open(order, _) => self.insert(order),
                    OrderBookEvent::Done(order_id, _) => self.remove(order_id),
                    OrderBookEvent::Change(order_id, new_size, _) => self.update(order_id, new_size),
                }
            },
            _ => Err(OrderBookError::StaleSequence(event))
        };
        self.update_sequence_and_stats(result, sequence);
    }

    /// update sequence, update stats if enabled (and error msgs if both stats and error msgs
    /// are enabled).
    fn update_sequence_and_stats(&mut self, result: Result<(), OrderBookError>, sequence: Sequence) {
        match result {
            Ok(()) => {
                self.last_sequence = sequence.clone();
                self.stats.as_mut().map(|stats| stats.events_processed += 1);
            },
            Err(OrderBookError::Outlier) => {
                self.last_sequence = sequence.clone();
                self.stats.as_mut().map(|stats| stats.events_not_processed += 1);
            },
            Err(error) => {
                warn!("Orderbook encountered error: {:?}", error);

                // don't update orderbook's sequence for stale sequence errors
                match error {
                    OrderBookError::StaleSequence(_) => {},
                    _ => {self.last_sequence = sequence.clone()}
                }

                self.stats.as_mut().map(|stats| {
                    stats.events_not_processed += 1;
                    stats.error_msgs
                        .as_mut()
                        .map(|map| {
                            map.push(format!("{:?} - sequence {:?} - {:?}", Utc::now(), self.last_sequence, error))
                        });
                });
            },
        }
    }

    /// make a NonNan price out of an order's price
    fn nan_check(order: &LimitOrder) -> Result<NonNan, OrderBookError> {
        NonNan::build(order.price().clone()).ok_or_else(|| OrderBookError::NanFloat(order.price().clone()))
    }

    /// check if order meets outlier condition
    fn check_new_outlier(&mut self, order: &LimitOrder) -> Result<(), OrderBookError> {
        if self.outlier_filter.is_some() {
            let top_level = self.top_level();
            self.outlier_filter.as_mut().unwrap().check(order, top_level)?;
            Ok(())
        } else { Ok(()) }
    }

    /// check if order id was one already encountered earlier as an outlier
    fn check_old_outlier(&self, order_id: &str) -> bool
    {
        if self.outlier_filter.is_some() {
            self.outlier_filter.as_ref().unwrap().outlier_ids.contains(&*order_id)
        } else { false }
    }

    /// remove outlier order from outlier filter's set
    fn remove_old_outlier(&mut self, order_id: &str) -> bool {
        if self.outlier_filter.is_some() {
            self.outlier_filter.as_mut().unwrap().outlier_ids.remove(&*order_id)
        } else { false }
    }

    /// Find order deque and push order to the back. If order deque does not exist,
    /// initialize one with order included and insert into the orderbook.
    ///
    /// Also inserts {order_id: (Side, Price)} pair into map to assist order retrieval.
    ///
    /// Skips insertion of orders with Nan floats.
    fn insert(&mut self, order: &LimitOrder) -> Result<(), OrderBookError> {
        let price= Self::nan_check(&order)?;
        self.check_new_outlier(&order)?;
        match order {
            LimitOrder::Bid(order) => {
                self.order_id_map.insert(order.id.clone(), (Side::Buy, price.clone()));
                let (_side, pos,maybe_deque) = self.get_deque_pos_mut(&Side::Buy, &price);
                match maybe_deque {
                    Ok(deque) => {
                        deque.push_back(order.clone());
                    },
                    Err(_) => {
                        self.bids.insert(pos, OrderDeque::build(order.clone()));
                    },
                };
                Ok(())
            }
            LimitOrder::Ask(order) => {
                self.order_id_map.insert(order.id.clone(), (Side::Sell, price.clone()));
                let (_side, pos,maybe_deque) = self.get_deque_pos_mut(&Side::Sell, &price);
                match maybe_deque {
                    Ok(deque) => {
                        deque.push_back(order.clone());
                    },
                    Err(_) => {
                        self.asks.insert(pos, OrderDeque::build(order.clone()));
                    },
                }
                Ok(())
            }
        }
    }

    /// Finds order's deque and removes it by index, and then removes it from order_id_map.
    /// If order deque is left with no orders, remove it too.
    fn remove(&mut self, order_id: &str) -> Result<(), OrderBookError> {
        let not_found_in_deque_msg = format!("{:?}", self.order_id_map.get_key_value(order_id));
        match self.get_deque_pos_mut_by_id(order_id) {
            Ok((side, deque_idx, maybe_deque)) => {
                match maybe_deque?.remove(order_id) {
                    Some(_order) => {
                        self.order_id_map.remove(order_id);
                        self.delete_deque_if_empty(side, deque_idx);
                        Ok(())
                    }
                    None => Err(OrderBookError::OrderNotFoundInDeque(not_found_in_deque_msg)),
                }
            }
            Err(OrderBookError::Outlier) => {
                self.remove_old_outlier(order_id);
                Err(OrderBookError::Outlier)
            },
            Err(e) => Err(e)
        }
    }

    /// Deletes an empty order queue from the bids or asks vector.
    fn delete_deque_if_empty(&mut self, side: Side, idx: usize) {
        match side {
            Side::Buy => {
                if self.bids[idx].deque.is_empty() {
                    self.bids.remove(idx);
                }
            }
            Side::Sell => {
                if self.asks[idx].deque.is_empty() {
                    self.asks.remove(idx);
                }
            }
        }
    }

    /// Finds mut ref to order and updates its size attribute
    fn update(&mut self, order_id: &str, new_size: &f64) -> Result<(), OrderBookError> {
        match self.get_order_mut(order_id) {
            Ok(order) => {
                order.size = new_size.to_owned();
                Ok(())
            },
            Err(e) => Err(e),
        }
    }

    /// Get reference to a deque by side and price
    fn get_deque_pos(&self, side: &Side, price: &NonNan) -> OrderDequePos<'_> {
        match side {
            Side::Buy => {
                match self.bids.binary_search_by_key(&Reverse(price.clone()), | order_deque| Reverse(order_deque.price)) {
                    Ok(pos) => (Side::Buy, pos.clone(), Ok(&self.bids[pos])),
                    Err(pos) => (Side::Buy, pos.clone(), Err(OrderBookError::MissingOrderDeque(price.clone()))),
                }
            }
            Side::Sell => {
                match self.asks.binary_search_by_key(price, | order_deque| order_deque.price) {
                    Ok(pos) => (Side::Sell, pos.clone(), Ok(&self.asks[pos])),
                    Err(pos) => (Side::Sell, pos.clone(), Err(OrderBookError::MissingOrderDeque(price.clone()))),
                }
            }
        }
    }

    /// Get mutable reference to a deque by side and price
    fn get_deque_pos_mut(&mut self, side: &Side, price: &NonNan) -> OrderDequePosMut<'_> {
        match side {
            Side::Buy => {
                match self.bids.binary_search_by_key(&Reverse(price.clone()), | order_deque| Reverse(order_deque.price)) {
                    Ok(pos) => (Side::Buy, pos.clone(), Ok(&mut self.bids[pos])),
                    Err(pos) => (Side::Buy, pos.clone(), Err(OrderBookError::MissingOrderDeque(price.clone()))),
                }
            }
            Side::Sell => {
                match self.asks.binary_search_by_key(price, | order_deque| order_deque.price) {
                    Ok(pos) => (Side::Sell, pos.clone(), Ok(&mut self.asks[pos])),
                    Err(pos) => (Side::Sell, pos.clone(), Err(OrderBookError::MissingOrderDeque(price.clone()))),
                }
            }
        }
    }

    /// Get a deque's position (side, index, ref) by an order's id.
    /// If outlier filter is enabled, check if the outlier filter has caught the order id
    /// as an outlier and return OrderbookError::Outlier if so.
    fn get_deque_pos_by_id(&self, order_id: &str) -> Result<OrderDequePos<'_>, OrderBookError> {
        if let Some(order_pos) = self.order_id_map.get(&*order_id) {
            let (side, price) = order_pos.clone();
            Ok(self.get_deque_pos(&side, &price))
        } else if self.check_old_outlier(&order_id) {
            Err(OrderBookError::Outlier)
        } else {
            Err(OrderBookError::OrderNotFoundInMap(order_id.to_owned()))
        }
    }

    /// Get a deque's mutable position (side, index, mut) by an order's id
    /// If outlier filter is enabled, check if the outlier filter has caught the order id
    /// as an outlier and return OrderbookError::Outlier if so.
    fn get_deque_pos_mut_by_id(&mut self, order_id: &str) -> Result<OrderDequePosMut<'_>, OrderBookError> {
        if let Some(order_pos) = self.order_id_map.get(&*order_id) {
            let (side, price) = order_pos.clone();
            Ok(self.get_deque_pos_mut(&side, &price))
        } else if self.check_old_outlier(&order_id) {
            Err(OrderBookError::Outlier)
        } else {
            Err(OrderBookError::OrderNotFoundInMap(order_id.to_owned()))
        }
    }

    /// Get reference to an order in the book by its id
    pub fn get_order_ref(&self, order_id: &str) -> Result<&AtomicOrder, OrderBookError> {
        let not_found_in_deque_msg = format!("{:?}", self.order_id_map.get_key_value(order_id));
        let (.., maybe_deque) = self.get_deque_pos_by_id(order_id)?;
        let deque = maybe_deque?;
        match deque.get_ref(order_id) {
            Some(order) => Ok(order),
            None => Err(OrderBookError::OrderNotFoundInDeque(not_found_in_deque_msg)),
        }
    }

    /// Get mutable reference to an order in the book by its id
    pub fn get_order_mut(&mut self, order_id: &str) -> Result<&mut AtomicOrder, OrderBookError> {
        let not_found_in_deque_msg = format!("{:?}", self.order_id_map.get_key_value(order_id));
        let (.., maybe_deque) = self.get_deque_pos_mut_by_id(order_id)?;
        let deque = maybe_deque?;
        match deque.get_mut(order_id) {
            Some(order) => Ok(order),
            None => Err(OrderBookError::OrderNotFoundInDeque(not_found_in_deque_msg))
        }
    }

    /// Return vector of (f64, f64, f64) tuples representing current snapshot of price, marginal
    /// order size (aggregate order size at each level)
    /// and running total of volume/liquidity (integral of price * order size)
    pub fn levels(&self, side: Side, depth: Option<usize>) -> Vec<(f64, f64, f64)> {
        match side {
            Side::Buy => {
                let scan = self.bids.iter().scan(0.0, |liquidity, deque| Option::from({
                    *liquidity += deque.price.0 * deque.size();
                    (deque.price.0, deque.size(), liquidity.clone())
                }));
                match depth {
                    Some(n) => scan.take(n).collect(),
                    None => scan.collect()
                }

            },
            Side::Sell => {
                let scan = self.asks.iter().scan(0.0, |liquidity, deque| Option::from({
                    *liquidity += deque.price.0 * deque.size();
                    (deque.price.0, deque.size(), liquidity.clone())
                }));
                match depth {
                    Some(n) => scan.take(n).collect(),
                    None => scan.collect()
                }
            },
        }
    }

    /// Return iterator that can iterate over every order in the book.
    pub fn iter(&self) -> Iter<'_> {
        let mut iter = Iter {
            side: Side::Buy,
            current_deque: None,
            bids_iter: self.bids.iter().rev().peekable(),
            asks_iter: self.asks.iter().peekable(),
            deque_iter: None,
        };

        if iter.bids_iter.peek().is_some() {
            iter.current_deque = iter.bids_iter.next();
        } else if iter.asks_iter.peek().is_some() {
            iter.current_deque = iter.asks_iter.next();
        };

        if let Some(order_deque) = iter.current_deque {
            let deque = &order_deque.deque;
            iter.deque_iter = Some(deque.iter());
        }

        iter
    }

    pub fn time_elapsed(&self) -> Duration {
        Utc::now() - self.start_time
    }

    pub fn hms(duration: Duration) -> String {
        let seconds = duration.num_seconds() % 60;
        let minutes = duration.num_minutes() % 60;
        let hours = duration.num_hours();
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }

    pub fn print_book(&self) {
        println!("-------All orders in book:-------");
        self.iter().for_each(|order| println!("{:?}", order));
        println!("----------------------------------")
    }

    pub fn print_info(&self, include_errors: bool) {
        println!("\n-------------------------------------------");
        println!("Orderbook Stats for {:?}", self.market);
        println!("Time elapsed: {}", Self::hms(self.time_elapsed()));
        println!("Last sequence: {:?}", self.last_sequence);
        println!("Best Bid/Ask: {:?} / {:?}", self.best_bid(), self.best_ask());
        println!("First 10 Bid Levels: {:?}", self.levels(Side::Buy, Some(10)));
        println!("First 10 Ask Levels: {:?}", self.levels(Side::Sell, Some(10)));
        println!("Bid/Ask/Total Counts: {:?} / {:?} / {:?}", self.bid_count(), self.ask_count(), self.len());
        self.print_stats(include_errors);
        self.print_outlier_stats();
        self.print_last_n_events();
        println!("---------------------------------------------\n");
    }

    pub fn print_stats(&self, include_errors: bool) {
        self.stats.as_ref().map(|stats| {
            println!("Events processed successfully: {:?}", stats.events_processed);
            println!("Events not processed: {:?}", stats.events_not_processed);

            if include_errors {
                match stats.error_msgs.as_ref() {
                    None => {
                        println!("Error messages not captured! Enable error message capturing by \
                        calling stats(track_errors: true) on the orderbook builder, when building \
                        an orderbook.");
                    }
                    Some(msgs) => {
                        println!("Error messages encountered: ");
                        msgs.iter().for_each(|msg| println!("{:?}", msg))
                    }
                }
            }

        });
    }

    pub fn get_error_msgs(&self) -> Option<&Vec<String>> {
        self.stats.as_ref().map(|stats| {
            stats.error_msgs.as_ref()
        }).flatten()
    }

    pub fn print_outlier_stats(&self) {
        self.outlier_filter.as_ref().map(|filter| {
            println!("Outlier factor used: {:?}", filter.outlier_factor);
            println!("Outliers encountered: {:?}", filter.outliers_encountered);
        });
    }

    pub fn print_last_n_events(&self) {
        if self.last_n_events.is_some() {
            println!("---------last {} events--------", self.last_n_events.as_ref().unwrap().len());
            self.last_n_events.as_ref().map(|events| {
                events.iter().for_each(|event| println!("{:?}", event))
            });
        }
    }
}

#[derive(Debug)]
pub struct Iter<'a> {
    side: Side,
    current_deque: Option<&'a OrderDeque>,
    bids_iter: Peekable<Rev<core::slice::Iter<'a, OrderDeque>>>,
    asks_iter: Peekable<core::slice::Iter<'a, OrderDeque>>,
    deque_iter: Option<std::collections::vec_deque::Iter<'a, AtomicOrder>>
}

impl<'a> Iterator for Iter<'a> {
    type Item = LimitOrder;

    fn next(&mut self) -> Option<Self::Item> {
        let mut atomic_order;
        let order;
        loop {
            match &self.side {
                Side::Buy => {
                    if let Some(_deque) = self.current_deque {
                        atomic_order = self.deque_iter.as_mut().unwrap().next();
                        if atomic_order.is_some() {
                            order = Some(LimitOrder::Bid(atomic_order.unwrap().clone()));
                            break
                        } else {
                            self.current_deque = self.bids_iter.next();
                            self.deque_iter = match self.current_deque {
                                Some(order_deque) => {
                                    Some(order_deque.deque.iter())  // todo: double-check this line in testing
                                },
                                None => None,
                            };
                            continue
                        }
                    } else {
                        self.side = Side::Sell;
                        self.current_deque = self.asks_iter.next();
                        if let Some(order_deque) = self.current_deque {
                            self.deque_iter = Some(order_deque.deque.iter());  // todo: this too
                        }
                    }
                }

                Side::Sell => {
                    if let Some(_deque) = self.current_deque {
                        atomic_order = self.deque_iter.as_mut().unwrap().next();
                        if atomic_order.is_some() {
                            order = Some(LimitOrder::Ask(atomic_order.unwrap().clone()));
                            break
                        } else {
                            self.current_deque = self.asks_iter.next();
                            self.deque_iter = match self.current_deque {
                                Some(order_deque) => {
                                    Some(order_deque.deque.iter())
                                },
                                None => None,
                            };
                            continue
                        }
                    } else {
                        order = None;
                        break
                    }
                }
            }
        }
        order
    }
}

/// builder to assist in instantiating an orderbook
#[derive(Debug)]
pub struct OrderbookBuilder {
    pub market: Option<Market>,
    pub outlier_filter: Option<SimpleOutlierFilter>,
    pub stats: Option<OrderbookStats>,
    pub last_n_events: Option<BoundedVecDeque<OrderBookEvent>>,
}

impl OrderbookBuilder {
    pub fn new() -> Self {
        Self {
            market: None,
            outlier_filter: None,
            stats: None,
            last_n_events: None,
        }
    }

    /// todo:
    pub fn market(self, market: Market) -> Self {
        Self {
            market: Some(market),
            ..self
        }
    }

    /// enable outlier filter using default factor (see DEFAULT_OUTLIER_FACTOR)
    pub fn outlier_filter_default(self) -> Self {
        Self {
            outlier_filter: Some(SimpleOutlierFilter::new(None)),
            ..self
        }
    }

    /// enable outlier factor using inputted factor
    pub fn outlier_filter(self, factor: f64) -> Self {
        Self {
            outlier_filter: Some(SimpleOutlierFilter::new(Some(factor))),
            ..self
        }
    }

    /// enable stats tracking
    ///
    /// pass true to collect any orderbook errors into a vector
    pub fn stats(self, track_errors: bool) -> Self {
        Self {
            stats: Some(OrderbookStats::new(track_errors)),
            ..self
        }
    }

    /// enable storing last n events
    pub fn last_n_events(self, n: usize) -> Self {
        Self {
            last_n_events: Some(BoundedVecDeque::new(n)),
            ..self
        }
    }

    /// build orderbook
    pub fn build(self) -> Result<OrderbookL3, OrderBookError> {
        let market = self.market.ok_or(OrderBookError::BuilderIncomplete("missing Market"))?;

        Ok(OrderbookL3 {
            market,
            last_sequence: 0,
            start_time: Utc::now(),
            bids: vec![],
            asks: vec![],
            order_id_map: HashMap::new(),
            outlier_filter: self.outlier_filter,
            stats: self.stats,
            last_n_events: self.last_n_events,
        })
    }
}

/// All orderbook errors that may occur in this module
#[derive(Error, Debug, PartialEq)]
pub enum OrderBookError {
    StaleSequence(OrderBookEvent),
    OrderNotFoundInMap(String),
    OrderNotFoundInDeque(String),
    MissingOrderDeque(NonNan),
    NanFloat(f64),
    Outlier,
    BuilderIncomplete(&'static str)
}

impl Display for OrderBookError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", &self)
    }
}

#[cfg(test)]
mod tests {
    use barter_integration::model::{
        Exchange,
        Instrument,
        InstrumentKind
    };
    use crate::ExchangeId;
    use super::*;
    use tracing_subscriber;

    #[test]
    pub fn orderbook_l3_basics() {
        tracing_subscriber::fmt().init();
        let instrument = Instrument::from(("eth", "usd", InstrumentKind::Spot));
        let exchange = Exchange::from(ExchangeId::Coinbase);
        let mut orderbook = OrderbookL3::builder()
            .market(Market::from((exchange.clone(), instrument.clone())))
            .stats(true)
            .outlier_filter_default()
            .build().unwrap();

        let invalid_events: Vec<OrderBookEvent> = vec![
            OrderBookEvent::Done("H".to_string(), 1),
            OrderBookEvent::Change("G".to_string(), 30.0, 1),
            OrderBookEvent::Done("F".to_string(), 1),
            OrderBookEvent::Done("ZZ".to_string(), 1),
        ];

        invalid_events.into_iter().for_each(|event| orderbook.process(event));

        // test empty book
        assert_eq!(orderbook.market, Market::from((exchange, instrument)));
        assert_eq!(orderbook.bids, vec![]);
        assert_eq!(orderbook.asks, vec![]);
        assert_eq!(orderbook.best_ask(), 0.0);
        assert_eq!(orderbook.best_bid(), 0.0);
        assert_eq!(orderbook.levels(Side::Buy, None), vec![]);
        assert_eq!(orderbook.levels(Side::Buy, None), vec![]);
        assert_eq!(orderbook.num_ask_levels(), 0);
        assert_eq!(orderbook.num_bid_levels(), 0);
        assert_eq!(orderbook.last_sequence, 0);


        // 3 ask levels, 4 bid levels post-insert
        let open_events= vec![
            OrderBookEvent::Open(LimitOrder::Ask(AtomicOrder { id: "A".to_string(), price: 1005.0, size: 20.0 }), 1),
            OrderBookEvent::Open(LimitOrder::Bid(AtomicOrder { id: "B".to_string(), price: 995.0, size: 5.0 }), 2),
            OrderBookEvent::Open(LimitOrder::Ask(AtomicOrder { id: "C".to_string(), price: 1006.0, size: 1.0 }), 3),
            OrderBookEvent::Open(LimitOrder::Bid(AtomicOrder { id: "D".to_string(), price: 994.0, size: 2.0 }), 4),
            OrderBookEvent::Open(LimitOrder::Ask(AtomicOrder { id: "E".to_string(), price: 1005.0, size: 0.25 }), 5),
            OrderBookEvent::Open(LimitOrder::Bid(AtomicOrder { id: "F".to_string(), price: 997.0, size: 10.0 }), 6),
            OrderBookEvent::Open(LimitOrder::Ask(AtomicOrder { id: "G".to_string(), price: 1001.0, size: 4.0 }), 7),
            OrderBookEvent::Open(LimitOrder::Bid(AtomicOrder { id: "H".to_string(), price: 996.0, size: 3.0 }), 8),
            OrderBookEvent::Open(LimitOrder::Ask(AtomicOrder { id: "I".to_string(), price: 1005.0, size: 10.0 }), 9),
            OrderBookEvent::Open(LimitOrder::Bid(AtomicOrder { id: "J".to_string(), price: 994.0, size: 6.0 }), 10),
        ];

        open_events.into_iter().for_each(|event| orderbook.process(event));

        assert_eq!(orderbook.get_order_ref("A").unwrap(), &AtomicOrder { id: "A".to_string(), price: 1005.0, size: 20.0 });
        assert_eq!(orderbook.get_order_ref("B").unwrap(), &AtomicOrder { id: "B".to_string(), price: 995.0, size: 5.0 });
        assert_eq!(orderbook.get_order_ref("C").unwrap(), &AtomicOrder { id: "C".to_string(), price: 1006.0, size: 1.0 });
        assert_eq!(orderbook.get_order_ref("D").unwrap(), &AtomicOrder { id: "D".to_string(), price: 994.0, size: 2.0 });
        assert_eq!(orderbook.get_order_ref("E").unwrap(), &AtomicOrder { id: "E".to_string(), price: 1005.0, size: 0.25 });
        assert_eq!(orderbook.get_order_ref("F").unwrap(), &AtomicOrder { id: "F".to_string(), price: 997.0, size: 10.0 });
        assert_eq!(orderbook.get_order_ref("G").unwrap(), &AtomicOrder { id: "G".to_string(), price: 1001.0, size: 4.0 });
        assert_eq!(orderbook.get_order_ref("H").unwrap(), &AtomicOrder { id: "H".to_string(), price: 996.0, size: 3.0 });
        assert_eq!(orderbook.get_order_ref("I").unwrap(), &AtomicOrder { id: "I".to_string(), price: 1005.0, size: 10.0 });
        assert_eq!(orderbook.get_order_ref("J").unwrap(), &AtomicOrder { id: "J".to_string(), price: 994.0, size: 6.0 });

        assert_eq!(orderbook.best_bid(), 997.0);
        assert_eq!(orderbook.best_ask(), 1001.0);

        let change_events = vec![
            OrderBookEvent::Change("A".to_string(), 30.0, 11),
            OrderBookEvent::Change("B".to_string(), 30.0, 12),
            OrderBookEvent::Change("C".to_string(), 30.0, 13),
            OrderBookEvent::Change("D".to_string(), 30.0, 14),
        ];

        change_events.into_iter().for_each(|event| orderbook.process(event));
        assert_eq!(orderbook.get_order_ref("A").unwrap().size, 30.0);
        assert_eq!(orderbook.get_order_ref("B").unwrap().size, 30.0);
        assert_eq!(orderbook.get_order_ref("C").unwrap().size, 30.0);
        assert_eq!(orderbook.get_order_ref("D").unwrap().size, 30.0);

        // 2 ask levels, 2 bid levels post-removal
        let close_events = vec![
            OrderBookEvent::Done("E".to_string(), 15),
            OrderBookEvent::Done("F".to_string(), 16),
            OrderBookEvent::Done("G".to_string(), 17),
            OrderBookEvent::Done("H".to_string(), 18),
        ];

        close_events.into_iter().for_each(|event| orderbook.process(event));
        assert_eq!(orderbook.get_order_ref("E"), Err(OrderBookError::OrderNotFoundInMap("E".to_string())));
        assert_eq!(orderbook.get_order_ref("F"), Err(OrderBookError::OrderNotFoundInMap("F".to_string())));
        assert_eq!(orderbook.get_order_ref("G"), Err(OrderBookError::OrderNotFoundInMap("G".to_string())));
        assert_eq!(orderbook.get_order_ref("H"), Err(OrderBookError::OrderNotFoundInMap("H".to_string())));

        // invalid events (out-of-sequence or missing)
        let invalid_events = vec![
            OrderBookEvent::Done("Z".to_string(), 18),
            OrderBookEvent::Open(LimitOrder::Bid(AtomicOrder { id: "D".to_string(), price: 994.0, size: 1000.0 }), 4),
            OrderBookEvent::Change("G".to_string(), 30.0, 14),
            OrderBookEvent::Done("ZZ".to_string(), 19),
        ];

        invalid_events.into_iter().for_each(|event| orderbook.process(event));

        // events with some outliers
        // 3 bid levels, 3 ask levels post-insert

        // best bid = 995 and best ask = 1005
        // with default outlier factor of 0.5
        // bid/ask cutoffs should be 497.5 / 1507.5

        let outlier_events = vec![
            OrderBookEvent::Open(LimitOrder::Bid(AtomicOrder { id: "K".to_string(), price: 500.0, size: 11.1 }), 19),
            OrderBookEvent::Open(LimitOrder::Bid(AtomicOrder { id: "L".to_string(), price: 400.0, size: 12.1 }), 20),
            OrderBookEvent::Open(LimitOrder::Bid(AtomicOrder { id: "M".to_string(), price: 300.0, size: 100.0 }), 21),
            OrderBookEvent::Open(LimitOrder::Ask(AtomicOrder { id: "N".to_string(), price: 1507.0, size: 13.1 }), 22),
            OrderBookEvent::Open(LimitOrder::Ask(AtomicOrder { id: "O".to_string(), price: 1600.0, size: 14.1 }), 23),
            OrderBookEvent::Open(LimitOrder::Ask(AtomicOrder { id: "P".to_string(), price: 10000.0, size: 100.0 }), 24),
            OrderBookEvent::Done("M".to_string(), 25),
            OrderBookEvent::Done("P".to_string(), 26),
        ];

        outlier_events.into_iter().for_each(|event| orderbook.process(event));

        let skipped_sequence_events = vec![
            OrderBookEvent::Open(LimitOrder::Ask(AtomicOrder { id: "Q".to_string(), price: 1005.0, size: 10.0 }), 28),
            OrderBookEvent::Open(LimitOrder::Bid(AtomicOrder { id: "R".to_string(), price: 994.0, size: 6.0 }), 30),
        ];

        skipped_sequence_events.into_iter().for_each(|event| orderbook.process(event));

        let mut expected_remaining = vec![
            LimitOrder::Ask(AtomicOrder { id: "A".to_string(), price: 1005.0, size: 30.0}),
            LimitOrder::Bid(AtomicOrder { id: "B".to_string(), price: 995.0, size: 30.0}),
            LimitOrder::Ask(AtomicOrder { id: "C".to_string(), price: 1006.0, size: 30.0}),
            LimitOrder::Bid(AtomicOrder { id: "D".to_string(), price: 994.0, size: 30.0}),
            LimitOrder::Ask(AtomicOrder { id: "I".to_string(), price: 1005.0, size: 10.0}),
            LimitOrder::Bid(AtomicOrder { id: "J".to_string(), price: 994.0, size: 6.0 }),
            LimitOrder::Bid(AtomicOrder { id: "K".to_string(), price: 500.0, size: 11.1}),
            LimitOrder::Ask(AtomicOrder { id: "N".to_string(), price: 1507.0, size: 13.1}),
            LimitOrder::Ask(AtomicOrder { id: "Q".to_string(), price: 1005.0, size: 10.0}),
            LimitOrder::Bid(AtomicOrder { id: "R".to_string(), price: 994.0, size: 6.0}),
        ];

        expected_remaining.sort_by_key(|order| NonNan::try_from(*order.price()).unwrap());

        for (idx, order) in orderbook.iter().enumerate() {
            assert_eq!(order, expected_remaining[idx])
        }

        assert_eq!(orderbook.get_order_ref("A").unwrap(), &AtomicOrder { id: "A".to_string(), price: 1005.0, size: 30.0 });
        assert_eq!(orderbook.get_order_ref("B").unwrap(), &AtomicOrder { id: "B".to_string(), price: 995.0, size: 30.0 });
        assert_eq!(orderbook.get_order_ref("C").unwrap(), &AtomicOrder { id: "C".to_string(), price: 1006.0, size: 30.0 });
        assert_eq!(orderbook.get_order_ref("D").unwrap(), &AtomicOrder { id: "D".to_string(), price: 994.0, size: 30.0 });
        assert_eq!(orderbook.get_order_ref("I").unwrap(), &AtomicOrder { id: "I".to_string(), price: 1005.0, size: 10.0 });
        assert_eq!(orderbook.get_order_ref("J").unwrap(), &AtomicOrder { id: "J".to_string(), price: 994.0, size: 6.0 });
        assert_eq!(orderbook.best_bid(), 995.0);
        assert_eq!(orderbook.best_ask(), 1005.0);
        assert_eq!(orderbook.num_ask_levels(), 3);
        assert_eq!(orderbook.num_bid_levels(), 3);
        assert_eq!(orderbook.outlier_filter.as_ref().unwrap().outliers_encountered, 4);

        orderbook.print_info(true);

    }
}
