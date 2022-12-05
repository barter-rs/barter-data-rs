use barter_integration::{
    model::{Exchange, Instrument, Market, Side},
    Event,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::model::orderbook::{AtomicOrder, OrderBookEvent};

/// Barter data structures that support subscribing to exchange specific market data.
///
/// eg/ `Subscription`, `SubscriptionId`, etc.
pub mod subscription;
pub mod orderbook;

/// Normalised Barter `MarketEvent` containing metadata about the included [`DataKind`] variant.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct MarketEvent {
    pub exchange_time: DateTime<Utc>,
    pub received_time: DateTime<Utc>,
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub kind: DataKind,
}

impl MarketEvent {
    pub fn market(&self) -> Market {
        Market::from((self.exchange.clone(), self.instrument.clone()))
    }

    pub fn sequence(&self) -> Option<u64> {
        match &self.kind {
            DataKind::Trade(trade) => Some(trade.sequence).flatten(),
            DataKind::OBEvent(ob_event) => Some(ob_event.sequence()),
            _ => None,
        }
    }
}

/// Defines the type of Barter [`MarketEvent`].
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum DataKind {
    Trade(PublicTrade),
    Candle(Candle),
    OrderBook(OrderBookL2Snapshot),
    OBEvent(OrderBookEvent),
    Liquidation(Liquidation),
}

/// Normalised Barter [`PublicTrade`] model.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PublicTrade {
    pub id: String,
    pub price: f64,
    pub quantity: f64,
    pub side: Side,
    pub sequence: Option<u64>,
}

impl PublicTrade {
    pub fn sequence(&self) -> Option<u64> {self.sequence}
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

/// Normalised Barter [`OrderBook`] snapshot.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OrderBookL2Snapshot {
    pub last_update_time: DateTime<Utc>,
    pub last_update_id: u64,
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
}

/// Normalised Barter [`OrderBook`] snapshot.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OrderBookL3Snapshot {
    pub last_update_time: DateTime<Utc>,
    pub sequence: u64,
    pub bids: Vec<AtomicOrder>,
    pub asks: Vec<AtomicOrder>,
}

/// Normalised Barter [`OrderBook`] [`Level`].
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Level {
    pub price: f64,
    pub quantity: f64,
}

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
