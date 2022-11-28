use barter_integration::{
    model::{Exchange, Instrument, Side},
    error::SocketError,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Todo: Rust docs & make FromIterator more generic for convenience ie/ Into<Market<Event>> etc.
pub struct MarketIter<Event>(pub Vec<Result<Market<Event>, SocketError>>);

impl<Event> FromIterator<Result<Market<Event>, SocketError>> for MarketIter<Event> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Result<Market<Event>, SocketError>>
    {
        Self(iter.into_iter().collect())
    }
}

/// Normalised Barter [`Market<Event>`](Self) containing metadata about the included `Event` variant.
///
/// Note: `Event` can be an enum if required.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Market<Event> {
    pub exchange_time: DateTime<Utc>,
    pub received_time: DateTime<Utc>,
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub event: Event,
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
    pub close_time: DateTime<Utc>,
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
