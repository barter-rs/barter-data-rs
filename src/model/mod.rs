use barter_integration::{
    model::{Exchange, Instrument, Side},
    Event,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::model::subscription::SubKindId;

/// Barter data structures that support subscribing to exchange specific market data.
///
/// eg/ `Subscription`, `SubscriptionId`, etc.
pub mod subscription;

/// Normalised Barter `MarketEvent` containing metadata about the included [`DataKind`] variant.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Market<Event> {
    pub exchange_time: DateTime<Utc>,
    pub received_time: DateTime<Utc>,
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub event: Event,
}

/// Defines the type of Barter [`MarketEvent`].
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum DataKind {
    Trade(PublicTrade),
    Candle(Candle),
    OrderBook(OrderBook),
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

/// Normalised Barter [`OrderBook`] snapshot.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OrderBook {
    pub last_update_time: DateTime<Utc>,
    pub last_update_id: u64,
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
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

impl SubKindId for PublicTrade {
    fn id() -> &'static str {
        "trade_public"
    }
}

impl SubKindId for OrderBook {
    fn id() -> &'static str {
        "order_book"
    }
}

impl SubKindId for Liquidation {
    fn id() -> &'static str {
        "liquidation"
    }
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

impl<T> From<Event<Market<T>>> for Market<T> {
    fn from(event: Event<Market<T>>) -> Self {
        event.payload
    }
}
