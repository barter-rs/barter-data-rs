use barter_integration::{
    error::SocketError,
    model::{Exchange, Instrument, Side},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Todo:
pub struct MarketIter<Event>(pub Vec<Result<Market<Event>, SocketError>>);

impl<Event> FromIterator<Result<Market<Event>, SocketError>> for MarketIter<Event> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Result<Market<Event>, SocketError>>,
    {
        Self(iter.into_iter().collect())
    }
}

/// Normalised Barter [`Market<Event>`](Self) containing metadata about the included `Event` variant.
///
/// Note: `Event` can be an enum if required.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct Market<Event> {
    pub exchange_time: DateTime<Utc>,
    pub received_time: DateTime<Utc>,
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub event: Event,
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