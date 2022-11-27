use std::collections::BTreeMap;

use barter_integration::{
    model::{Exchange, Instrument, Side},
    Event,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ordered_float::OrderedFloat;

/// Barter data structures that support subscribing to exchange specific market data.
///
/// eg/ `Subscription`, `SubscriptionId`, etc.
pub mod subscription;

/// Normalised Barter `MarketEvent` containing metadata about the included [`DataKind`] variant.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct MarketEvent {
    pub exchange_time: DateTime<Utc>,
    pub received_time: DateTime<Utc>,
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub kind: DataKind,
}

/// Defines the type of Barter [`MarketEvent`].
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum DataKind {
    Trade(PublicTrade),
    Candle(Candle),
    /// Level 2 orderbook snapshot
    OrderBook(OrderBook),
    /// Level 2 orderbook update
    OrderBookL2Update(OrderBookL2Update),
    Liquidation(Liquidation),
}

/// Normalised Barter [`PublicTrade`] model.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PublicTrade {
    pub id: String,
    pub price: f64,
    pub quantity: f64,
    pub side: Side,
}

/// Normalised Barter OHLCV [`Candle`] model.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Candle {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub trade_count: u64,
}

/// Normalised Barter level 2 [`OrderBook`] snapshot.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OrderBook {
    pub last_update_id: u64,
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
}

/// Normalised Barter level 2 [`OrderBook`] snapshot.
#[derive(Clone, Debug, PartialEq, PartialOrd,)]
pub struct OrderBookBTreeMap {
    pub last_update_id: u64,
    pub bids: BTreeMap<OrderedFloat<f64>, f64>,
    pub asks: BTreeMap<OrderedFloat<f64>, f64>,
}

impl OrderBookBTreeMap {
    pub fn new() -> Self {
        let bids = BTreeMap::new();
        let asks = BTreeMap::new();
        let last_update_id = 0;

        OrderBookBTreeMap { last_update_id, bids, asks }
    }

    /// Apply an [`OrderBookL2Update`] to the order book.
    /// Assumes that sanity checks are done by the end user.
    pub fn apply_update(&mut self, update: &OrderBookL2Update) {
        match update.update_type {
            L2UpdateType::UpdateLevel { price, quantity } => {
                match update.book_side {
                    OBSide::Bid => {
                        self.bids.insert(OrderedFloat(price), quantity);
                    },
                    OBSide::Ask => {
                        self.asks.insert(OrderedFloat(price), quantity);
                    }
                }
            },
            L2UpdateType::RemoveLevel { price } => {
                match update.book_side {
                    OBSide::Bid => {
                        self.bids.remove(&OrderedFloat(price));
                    },
                    OBSide::Ask => {
                        self.asks.remove(&OrderedFloat(price));
                    }
                }
            }
        }
        self.last_update_id = update.sequence_num;
    }

    pub fn get_best_bid(&self) -> Option<f64> {
        self.bids.iter().next_back().map(|v| v.0.into_inner())
    }

    pub fn get_best_ask(&self) -> Option<f64> {
        self.asks.keys().next().map(|v| v.0)
    }
}

/// Normalised Barter [`OrderBook`] [`Level`].
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Level {
    pub price: f64,
    pub quantity: f64,
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OrderBookL2Update {
    pub sequence_num: u64,
    pub book_side: OBSide,
    pub update_type: L2UpdateType,
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum OBSide {
    Bid,
    Ask,
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum L2UpdateType {
    RemoveLevel {
        /// The price level to be removed
        price: f64,
    },
    UpdateLevel {
        /// The price level
        price: f64,
        /// The quantity of this level
        quantity: f64,
    },
}

/// Normalized Barter Level 2 [ `Order`]

/// Normalised Barter [`Liquidation`] model.
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Liquidation {
    pub side: Side,
    pub price: f64,
    pub quantity: f64,
    pub time: DateTime<Utc>,
}

impl<T> From<(T, T)> for Level
where
    T: Into<f64>,
{
    fn from((price, quantity): (T, T)) -> Self {
        Self::new(price, quantity)
    }
}

impl Level {
    pub fn new<T>(price: T, quantity: T) -> Self
    where
        T: Into<f64>,
    {
        Self {
            price: price.into(),
            quantity: quantity.into(),
        }
    }
}



impl From<Event<MarketEvent>> for MarketEvent {
    fn from(event: Event<MarketEvent>) -> Self {
        event.payload
    }
}
