use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ta::{Close, High, Low, Open, Volume};

#[derive(Debug, Deserialize, Serialize, PartialOrd, PartialEq, Clone)]
pub enum MarketData {
    Trade(Trade),
    Candle(Candle),
}

/// Normalised Trade model to be returned from an ExchangeClient implementor instance.
#[derive(Debug, Deserialize, Serialize, PartialOrd, PartialEq, Clone)]
pub struct Trade {
    pub trade_id: String,
    pub timestamp: DateTime<Utc>,
    pub ticker: String,
    pub price: f64,
    pub quantity: f64,
    pub buyer: BuyerType,
}

/// Defines if the buyer in a [Trade] is a market maker.
#[derive(Debug, Deserialize, Serialize, PartialOrd, PartialEq, Clone)]
pub enum BuyerType {
    MarketMaker,
    Taker,
}

/// Defines the possible intervals that a [Candle] represents.
#[derive(Debug, Deserialize, Serialize, PartialOrd, PartialEq, Clone)]
pub enum Interval {
    Minute1,
    Minute3,
    Minute5,
    Minute15,
    Minute30,
    Hour1,
    Hour2,
    Hour4,
    Hour6,
    Hour8,
    Hour12,
    Day1,
    Day3,
    Week1,
    Month1,
}

/// Normalised OHLCV data from an [Interval] with the associated [DateTime] UTC timestamp;
#[derive(Debug, Deserialize, Serialize, PartialOrd, PartialEq, Clone)]
pub struct Candle {
    pub start_timestamp: DateTime<Utc>,
    pub end_timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub trade_count: u64,
}

impl Default for Candle {
    fn default() -> Self {
        Self {
            start_timestamp: Utc::now(),
            end_timestamp: Utc::now(),
            open: 1000.0,
            high: 1100.0,
            low: 900.0,
            close: 1050.0,
            volume: 1000000000.0,
            trade_count: 100,
        }
    }
}

impl Open for Candle {
    fn open(&self) -> f64 {
        self.open
    }
}

impl High for Candle {
    fn high(&self) -> f64 {
        self.high
    }
}

impl Low for Candle {
    fn low(&self) -> f64 {
        self.low
    }
}

impl Close for Candle {
    fn close(&self) -> f64 {
        self.close
    }
}

impl Volume for Candle {
    fn volume(&self) -> f64 {
        self.volume
    }
}
