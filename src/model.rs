use barter_integration::{
    Instrument,
};
use std::{
    fmt::Debug,
};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Normalised Barter market data types.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum MarketData {
    Trade(Trade),
    Candle,
    Kline,
    OrderBook,
}

/// Normalised public [`Trade`] model.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Trade {
    pub id: String,
    pub exchange: String,
    pub instrument: Instrument,
    pub received_timestamp: DateTime<Utc>,
    pub exchange_timestamp: DateTime<Utc>,
    pub price: f64,
    pub quantity: f64,
    pub direction: Direction,
}

/// Direction of a [`Trade`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum Direction {
    #[serde(alias = "buy")]
    Buy,
    #[serde(alias = "sell")]
    Sell
}