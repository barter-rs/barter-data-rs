#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use std::borrow::Borrow;
// standard
use std::collections::{HashMap, HashSet, VecDeque};
use std::cmp::{Ordering, Reverse};
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::iter::{Peekable, Rev};
// external
use barter_integration::model::{Exchange, Instrument, Market, Side};
use chrono::Utc;
use thiserror::Error;
use serde::{Deserialize, Serialize};
// internal
use crate::model::de_floats;

const DEFAULT_OUTLIER_FACTOR: f64 = 2.0;
const DEFAULT_BEST_BID: f64 = 0.0;
const DEFAULT_BEST_ASK: f64 = 0.0;

pub type NewSize = f64;
pub type Sequence = u64;
pub type OrderDequePos<'a> = (Side, usize, Result<&'a OrderDeque, OrderbookError>);
pub type OrderDequePosMut<'a> = (Side, usize, Result<&'a mut OrderDeque, OrderbookError>);
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

    pub fn get(&self, market: &Market) -> Option<&OrderbookL3>{
        self.map.get(market)
    }

    pub fn get_mut(&mut self, market: &Market) -> Option<&mut OrderbookL3> {
        self.map.get_mut(market)
    }
}

/// Todo:
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub enum OrderbookEvent {
    Received(Order, Sequence),
    Open(Order, Sequence),
    Done(String, Sequence),
    Change(String, NewSize, Sequence),
}

impl OrderbookEvent {
    pub fn sequence(&self) -> Sequence {
        match self {
            OrderbookEvent::Received(_, seq) => seq.clone(),
            OrderbookEvent::Open(_, seq) => seq.clone(),
            OrderbookEvent::Done(_, seq) => seq.clone(),
            OrderbookEvent::Change(_, _, seq) => seq.clone(),
        }
    }
}

/// Todo:
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    Limit,
    Market,
}

/// Todo:
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub enum Order {
    Bid(AtomicOrder, OrderType),
    Ask(AtomicOrder, OrderType),
}

impl Order {
    pub fn id(&self) -> &str {
        match self {
            Order::Bid(order, ..) => &order.id,
            Order::Ask(order, ..) => &order.id,
        }
    }

    pub fn price(&self) -> &f64 {
        match self {
            Order::Bid(order, ..) => &order.price,
            Order::Ask(order, ..) => &order.price,
        }
    }

    pub fn side(&self) -> Side {
        match self {
            Order::Bid(..) => Side::Buy,
            Order::Ask(..) => Side::Sell,
        }
    }
}

/// Todo:
/// Most basic order struct
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

    /// remove an order by its index
    fn remove(&mut self, order_idx: usize) -> Option<AtomicOrder> {
        self.deque.remove(order_idx)
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
/// the current best bids and asks in the orderbook. For bids, this
/// cutoff is best_bid / outlier factor, and for asks this cutoff is
/// best_bid * outlier_factor.
///
/// Enable using the outlier_filter or outlier_filter_default methods in the orderbook builder.
#[derive(Clone, Debug)]
pub struct SimpleOutlierFilter {
    pub outlier_factor: f64,
    pub outlier_ids: HashSet<String>,
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
        }
    }

    /// checks if an incoming order is an outlier based on its price in relation to
    /// the best bid, best ask and the outlier factor.
    ///
    /// For all outliers, add order id to hashset and return OrderbookError::Outlier.
    pub fn check(&mut self, order: &Order, top_level: TopLevel) -> Result<(), OrderbookError> {
        match order.side() {
            Side::Buy => {
                // initial conditions (empty book)
                if top_level.0 == DEFAULT_BEST_BID {
                    Ok(())
                } else {
                    // update cutoff based on best bid and compare
                    match order.price().partial_cmp(&(top_level.0 / self.outlier_factor)) {
                        Some(Ordering::Less) => {
                            self.outlier_ids.insert(order.id().to_owned());
                            Err(OrderbookError::Outlier)
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
                    // update cutoff based on best ask and compare
                    match order.price().partial_cmp(&(top_level.1 * self.outlier_factor)) {
                        Some(Ordering::Greater) => {
                            self.outlier_ids.insert(order.id().to_owned());
                            Err(OrderbookError::Outlier)
                        },
                        _ => Ok(())
                    }
                }
            }
        }
    }
}

/// Orderbook stats container
///
/// Error messages can be enabled by passing
#[derive(Clone, Debug)]
pub struct OrderbookStats {
    pub events_processed: u64,
    pub events_not_processed: u64,
    pub error_msgs: Option<HashSet<String>>,
}

impl OrderbookStats {
    pub fn new(track_errors: bool) -> Self {
        Self {
            events_processed: 0,
            events_not_processed: 0,
            error_msgs: match track_errors {
                true => Some(HashSet::<String>::new()),
                false => None,
            },
        }
    }
}

/// Todo: consider alternative data structures for bids and asks
#[derive(Clone, Debug)]
pub struct OrderbookL3 {
    // info
    pub market: Option<Market>,
    pub last_sequence: u64,

    // data structures
    pub bids: Vec<OrderDeque>,
    pub asks: Vec<OrderDeque>,
    // todo: consider replacing (Side, NonNan) with raw pointer or Arc Mutex to OrderDeque
    pub order_id_map: HashMap<String, (Side, NonNan)>,

    pub outlier_filter: Option<SimpleOutlierFilter>,
    pub stats: Option<OrderbookStats>,
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

    /// process an OrderbookEvent
    pub fn process(&mut self, event: OrderbookEvent) {
        let sequence = event.sequence();
        let result: Result<(), OrderbookError> = match &sequence.cmp(&self.last_sequence) {
            Ordering::Greater => {
                match &event {
                    // todo: received orders do not change state of orderbook but may be used to model
                    // market order impacts before ensuing order limit close messages arrive
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

    /// update sequence, update stats if enabled (and error msgs if both stats and error msgs
    /// are enabled).
    fn update_sequence_and_stats(&mut self, result: Result<(), OrderbookError>, sequence: Sequence) {
        match result {
            Ok(()) => {
                self.last_sequence = sequence.clone();
                self.stats.as_mut().map(|stats| stats.events_processed += 1);
            },
            Err(OrderbookError::Outlier) => {
                self.last_sequence = sequence.clone();
                self.stats.as_mut().map(|stats| stats.events_not_processed += 1);
            },
            Err(error) => {
                self.stats.as_mut().map(|stats| {
                    stats.events_not_processed += 1;
                    stats.error_msgs
                        .as_mut()
                        .map(|map| {
                            map.insert(format!("{:?} - {:?} - {:?}", Utc::now(), sequence, error))
                        });
                });
            },
        }
    }

    /// make a NonNan price out of an order's price
    fn nan_check(order: &Order) -> Result<NonNan, OrderbookError> {
        NonNan::build(order.price().clone()).ok_or_else(|| OrderbookError::NanFloat(order.clone()))
    }

    /// check if order meets outlier condition
    fn check_new_outlier(&mut self, order: &Order) -> Result<(), OrderbookError> {
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
    fn insert(&mut self, order: &Order) -> Result<(), OrderbookError> {
        let price= Self::nan_check(&order)?;
        self.check_new_outlier(&order)?;
        match order {
            Order::Bid(order, _) => {
                self.order_id_map.insert(order.id.clone(), (Side::Buy, price.clone()));
                let (_side, pos,maybe_deque) = self.get_deque_pos_mut(&Side::Buy, &price);
                Ok(match maybe_deque {
                    Ok(deque) => deque.push_back(order.clone()),
                    Err(e) => self.bids.insert(pos, OrderDeque::build(order.clone())),
                })
            }
            Order::Ask(order, _) => {
                self.order_id_map.insert(order.id.clone(), (Side::Sell, price.clone()));
                let (_side, pos,maybe_deque) = self.get_deque_pos_mut(&Side::Sell, &price);
                Ok(match maybe_deque {
                    Ok(deque) => deque.push_back(order.clone()),
                    Err(e) => self.asks.insert(pos, OrderDeque::build(order.clone())),
                })
            }
        }
    }

    /// Finds order's deque and removes it by index, and then removes it from order_id_map.
    /// If order deque is left with no orders, remove it too.
    fn remove(&mut self, order_id: &str) -> Result<(), OrderbookError> {
        let (side, idx, maybe_deque) = self.get_deque_pos_mut_by_id(order_id)?;
        let deque = maybe_deque?;
        match deque.remove(idx) {
            Some(order) => {
                self.order_id_map.remove(order_id);
                self.delete_deque_if_empty(side, idx);
                Ok(())
            }
            None => Err(OrderbookError::OrderNotFoundInDeque(order_id.to_owned())),
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
    fn update(&mut self, order_id: &str, new_size: &f64) -> Result<(), OrderbookError> {
        println!("updating order_id {}", order_id);
        match self.get_order_mut(order_id) {
            Ok(order) => {
                order.size = new_size.to_owned();
                println!("order {}'s new size {}", order_id, order.size);
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
                    Err(pos) => (Side::Buy, pos.clone(), Err(OrderbookError::MissingOrderDeque(price.clone()))),
                }
            }
            Side::Sell => {
                match self.asks.binary_search_by_key(price, | order_deque| order_deque.price) {
                    Ok(pos) => (Side::Sell, pos.clone(), Ok(&self.asks[pos])),
                    Err(pos) => (Side::Sell, pos.clone(), Err(OrderbookError::MissingOrderDeque(price.clone()))),
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
                    Err(pos) => (Side::Buy, pos.clone(), Err(OrderbookError::MissingOrderDeque(price.clone()))),
                }
            }
            Side::Sell => {
                match self.asks.binary_search_by_key(price, | order_deque| order_deque.price) {
                    Ok(pos) => (Side::Sell, pos.clone(), Ok(&mut self.bids[pos])),
                    Err(pos) => (Side::Sell, pos.clone(), Err(OrderbookError::MissingOrderDeque(price.clone()))),
                }
            }
        }
    }

    /// Get a deque's position (side, index, ref) by an order's id.
    /// If outlier filter is enabled, check if the outlier filter has caught the order id
    /// as an outlier.
    fn get_deque_pos_by_id(&self, order_id: &str) -> Result<OrderDequePos<'_>, OrderbookError> {
        if let Some(order_pos) = self.order_id_map.get(&*order_id) {
            let (side, price) = order_pos.clone();
            Ok(self.get_deque_pos(&side, &price))
        } else if self.check_old_outlier(&order_id) {
            Err(OrderbookError::Outlier)
        } else {
            Err(OrderbookError::OrderNotFoundInMap(order_id.to_owned()))
        }
    }

    /// Get a deque's mutable position (side, index, mut) by an order's id
    fn get_deque_pos_mut_by_id(&mut self, order_id: &str) -> Result<OrderDequePosMut<'_>, OrderbookError> {
        if let Some(order_pos) = self.order_id_map.get(&*order_id) {
            let (side, price) = order_pos.clone();
            Ok(self.get_deque_pos_mut(&side, &price))
        } else if self.check_old_outlier(&order_id) {
            Err(OrderbookError::Outlier)
        } else {
            Err(OrderbookError::OrderNotFoundInMap(order_id.to_owned()))
        }
    }

    /// Get reference to an order in the book by its id
    fn get_order_ref(&self, order_id: &str) -> Result<&AtomicOrder, OrderbookError> {
        let (side, idx, maybe_deque) = self.get_deque_pos_by_id(order_id)?;
        let deque = maybe_deque?;
        match deque.get_ref(order_id) {
            Some(order) => Ok(order),
            None => Err(OrderbookError::OrderNotFoundInDeque(order_id.to_owned()))
        }
    }

    /// Get mutable reference to an order in the book by its id
    fn get_order_mut(&mut self, order_id: &str) -> Result<&mut AtomicOrder, OrderbookError> {
        let (side, idx, maybe_deque) = self.get_deque_pos_mut_by_id(order_id)?;
        let deque = maybe_deque?;
        match deque.get_mut(order_id) {
            Some(order) => Ok(order),
            None => Err(OrderbookError::OrderNotFoundInDeque(order_id.to_owned()))
        }
    }

    /// Return vector of (f64, f64, f64) tuples representing current snapshot of price, marginal
    /// order size (aggregate order size at each level)
    /// and running total of volume/liquidity (integral of price * order size)
    fn levels(&self, side: Side, depth: Option<usize>) -> Vec<(f64, f64, f64)> {
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
    fn iter(&self) -> Iter<'_> {
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

    pub fn print_error_msgs(&self) {
        self.stats.as_ref().map(|stats| {
            stats.error_msgs.as_ref().map(|msgs| {
                msgs.iter().for_each(|str| println!("{}", str))
            })
        });
    }
}

pub struct Iter<'a> {
    side: Side,
    current_deque: Option<&'a OrderDeque>,
    bids_iter: Peekable<Rev<core::slice::Iter<'a, OrderDeque>>>,
    asks_iter: Peekable<core::slice::Iter<'a, OrderDeque>>,
    deque_iter: Option<std::collections::vec_deque::Iter<'a, AtomicOrder>>
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a AtomicOrder;

    fn next(&mut self) -> Option<Self::Item> {
        let mut result;
        loop {
            match &self.side {
                Side::Buy => {
                    if let Some(_deque) = self.current_deque {
                        result = self.deque_iter.as_mut().unwrap().next();
                        if result.is_some() {
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
                    if let Some(_node) = self.current_deque {
                        result = self.deque_iter.as_mut().unwrap().next();
                        if result.is_some() {
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
                        result = None;
                        break
                    }
                }
            }
        }
        result
    }
}

/// builder to assist in instantiating an orderbook
pub struct OrderbookBuilder {
    pub market: Option<Market>,
    pub outlier_filter: Option<SimpleOutlierFilter>,
    pub stats: Option<OrderbookStats>,
}

impl OrderbookBuilder {
    pub fn new() -> Self {
        Self {
            market: None,
            outlier_filter: None,
            stats: None,
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
    /// pass true to track_errors to collect any orderbook errors into a set
    pub fn stats(self, track_errors: bool) -> Self {
        Self {
            stats: Some(OrderbookStats::new(track_errors)),
            ..self
        }
    }

    /// build orderbook
    pub fn build(self) -> OrderbookL3 {
        OrderbookL3 {
            market: self.market,
            last_sequence: 0,
            bids: vec![],
            asks: vec![],
            order_id_map: HashMap::new(),
            outlier_filter: self.outlier_filter,
            stats: self.stats,
        }
    }
}

/// All orderbook errors that may occur in this module
#[derive(Error, Debug)]
pub enum OrderbookError {
    OutOfSequence(OrderbookEvent),
    OrderNotFoundInMap(String),
    OrderNotFoundInDeque(String),
    MissingOrderDeque(NonNan),
    NanFloat(Order),
    Outlier,
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
    use crate::model::{DataKind, DataKind::OrderBookEvent};
    use crate::orderbook::Order;
    use crate::orderbook::OrderbookEvent::{Change, Done, Open};
    use super::*;
    use rand::Rng;
    use crate::orderbook::{OrderbookEvent, OrderType};

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
        let orderbook = OrderbookL3::builder().build();

        todo!()

    }

    #[test]
    pub fn orderbook_l3_basics() {
        let instrument = Instrument::from(("eth", "usd", InstrumentKind::Spot));
        let exchange = Exchange::from(ExchangeId::Coinbase);
        let mut orderbook = OrderbookL3::builder()
            .stats(true)
            .build();

        let invalid_events: Vec<OrderbookEvent> = vec![
            Done("H".to_string() , 18),
            Change("G".to_string(), 30.0, 14),
            Done("F".to_string() , 17),
            Done("ZZ".to_string() , 100),
        ];

        invalid_events.into_iter().for_each(|event| orderbook.process(event));
        orderbook.print_error_msgs();

        // test empty book
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

        open_events.into_iter().for_each(|event| orderbook.process(event));
        println!("{:?}", orderbook.levels(Side::Sell, None));
        println!("{:?}", orderbook.levels(Side::Buy, None));
        assert_eq!(orderbook.best_bid(), 997.0);
        assert_eq!(orderbook.best_ask(), 1001.0);

        let change_events = vec![
            Change("A".to_string(), 30.0, 11),
            Change("B".to_string(), 30.0, 12),
            Change("C".to_string(), 30.0, 13),
            Change("D".to_string(), 30.0, 14),
        ];

        change_events.into_iter().for_each(|event| orderbook.process(event));
        println!("{:?}", orderbook.levels(Side::Sell, None));
        println!("{:?}", orderbook.levels(Side::Buy, None));
        assert_eq!(orderbook.get_order_ref("A").unwrap().size, 30.0);
        assert_eq!(orderbook.get_order_ref("B").unwrap().size, 30.0);
        assert_eq!(orderbook.get_order_ref("C").unwrap().size, 30.0);
        assert_eq!(orderbook.get_order_ref("D").unwrap().size, 30.0);

        // 2 ask levels, 2 bid levels post-removal
        let close_events = vec![
            Done("E".to_string(), 15),
            Done("F".to_string(), 16),
            Done("G".to_string(), 17),
            Done("H".to_string(), 18),
        ];

        // invalid events (out-of-sequence or missing)
        let invalid_events = vec![
            Done("J".to_string() , 18),
            Open(Order::Bid(AtomicOrder { id: "D".to_string(), price: 994.0, size: 2.0 }, OrderType::Limit), 4),
            Change("G".to_string(), 30.0, 14),
            Done("ZZ".to_string(), 19),
        ];

        assert_eq!(orderbook.best_bid(), 995.0);
        assert_eq!(orderbook.best_ask(), 1005.0);
        assert_eq!(orderbook.num_ask_levels(), 2);
        assert_eq!(orderbook.num_bid_levels(), 2);
        println!("{:?}", orderbook.num_bid_levels());
        println!("{:?}", orderbook.bids);
        println!("{:?}", orderbook.asks);
        println!("{:?}", orderbook.levels(Side::Buy, None));
        println!("{:?}", orderbook.levels(Side::Sell, None));

    }

}
