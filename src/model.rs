#![allow(dead_code)]

use crate::ExchangeId;
use barter_integration::{
    error::SocketError,
    Event,
    model::{Exchange, Instrument, InstrumentKind, Market, Side, SubscriptionId, Symbol},
    protocol::websocket::WsMessage, Validator,
};
use chrono::{DateTime, Utc};
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::{
    collections::HashMap,
    fmt::{Debug, Display, Formatter},
    ops::{Deref, DerefMut},
};
use crate::orderbook::{OrderbookEvent};

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
    OrderBookL2(OrderbookL2),
    OrderBookDelta(OrderBookDelta),
    OrderBookEvent(OrderbookEvent),
    // OrderBookL3(OrderBookL3Snapshot),
}

impl DataKind {
    pub fn sequence(&self) -> &u64 {
        match self {
            DataKind::Trade(trade) => {&trade.sequence}
            DataKind::Candle(_candle) => {&0}  // todo: derive this?
            DataKind::OrderBookL2(orderbook) => {&orderbook.last_update_id}
            DataKind::OrderBookDelta(delta) => {&delta.update_id}
            DataKind::OrderBookEvent(event) => {
                match event {
                    OrderbookEvent::Received(_, sequence) => {sequence}
                    OrderbookEvent::Open(_, sequence) => {sequence}
                    OrderbookEvent::Done(_, sequence) => {sequence}
                    OrderbookEvent::Change(_, _, sequence) => {sequence}
                }
            },
            // DataKind::OrderBookL3(orderbook) => {&orderbook.sequence}
        }
    }
}

/// Normalised Barter [`PublicTrade`] model.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PublicTrade {
    pub id: String,
    pub price: f64,
    pub quantity: f64,
    pub side: Side,
    pub sequence: u64,
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

/// Todo:
///  - Rust docs
///  - Will have to add fields to support generic delta updates...
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OrderbookL2 {
    pub last_update_id: u64,
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
}

/// Todo: Rust docs
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Level {
    #[serde(deserialize_with = "crate::exchange::de_str")]
    pub price: f64,
    #[serde(deserialize_with = "crate::exchange::de_str")]
    pub quantity: f64,
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

/// Todo: Rust docs
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OrderBookDelta {
    pub update_id: u64,
    pub bid_deltas: Vec<LevelDelta>,
    pub ask_deltas: Vec<LevelDelta>,
}

/// Todo: Rust docs
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct LevelDelta {
    #[serde(deserialize_with = "crate::exchange::de_str")]
    pub price: f64,
    #[serde(deserialize_with = "crate::exchange::de_str")]
    pub quantity: f64,
}

impl<T> From<(T, T)> for LevelDelta
where
    T: Into<f64>,
{
    fn from((price, quantity): (T, T)) -> Self {
        Self::new(price, quantity)
    }
}

impl LevelDelta {
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

/// Barter [`Subscription`] used to subscribe to a market [`SubKind`] for a particular
/// [`Exchange`]'s [`Instrument`].
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Subscription {
    pub exchange: ExchangeId,
    #[serde(flatten)]
    pub instrument: Instrument,
    #[serde(alias = "type")]
    pub kind: SubKind,
}

impl Validator for &Subscription {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match self.instrument.kind {
            InstrumentKind::Spot if self.exchange.supports_spot() => Ok(self),
            InstrumentKind::FuturePerpetual if self.exchange.supports_futures() => Ok(self),
            other => Err(SocketError::Unsupported {
                entity: self.exchange.as_str(),
                item: other.to_string(),
            }),
        }
    }
}

impl Debug for Subscription {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}{}", self.exchange, self.kind, self.instrument)
    }
}

impl Display for Subscription {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<S> From<(ExchangeId, S, S, InstrumentKind, SubKind)> for Subscription
where
    S: Into<Symbol>,
{
    fn from(
        (exchange, base, quote, instrument_kind, kind): (ExchangeId, S, S, InstrumentKind, SubKind),
    ) -> Self {
        Self::new(exchange, (base, quote, instrument_kind), kind)
    }
}

impl<I> From<(ExchangeId, I, SubKind)> for Subscription
where
    I: Into<Instrument>,
{
    fn from((exchange, instrument, stream): (ExchangeId, I, SubKind)) -> Self {
        Self::new(exchange, instrument, stream)
    }
}

impl From<Subscription> for Market {
    fn from(subscription: Subscription) -> Self {
        Self::new(subscription.exchange, subscription.instrument)
    }
}

impl Subscription {
    /// Constructs a new [`Subscription`] using the provided configuration.
    pub fn new<I>(exchange: ExchangeId, instrument: I, kind: SubKind) -> Self
    where
        I: Into<Instrument>,
    {
        Self {
            exchange,
            instrument: instrument.into(),
            kind,
        }
    }
}

/// Possible Barter [`Subscription`] types.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubKind {
    Trade,
    Candle(Interval),
    OrderBookL2,
    OrderBookL3,
}

impl Display for SubKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SubKind::Trade => "trade".to_owned(),
                SubKind::Candle(interval) => format!("candle_{}", interval),
                SubKind::OrderBookL2 => "order_book_l2".to_owned(),
                SubKind::OrderBookL3 => "order_book_l3".to_owned(),
            }
        )
    }
}

/// Barter time interval used for specifying the interval of a [`SubKind::Candle`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum Interval {
    #[serde(alias = "1m")]
    Minute1,
    #[serde(alias = "3m")]
    Minute3,
    #[serde(alias = "5m")]
    Minute5,
    #[serde(alias = "15m")]
    Minute15,
    #[serde(alias = "30m")]
    Minute30,
    #[serde(alias = "1h")]
    Hour1,
    #[serde(alias = "2h")]
    Hour2,
    #[serde(alias = "4h")]
    Hour4,
    #[serde(alias = "6h")]
    Hour6,
    #[serde(alias = "8h")]
    Hour8,
    #[serde(alias = "12h")]
    Hour12,
    #[serde(alias = "1d")]
    Day1,
    #[serde(alias = "3d")]
    Day3,
    #[serde(alias = "1w")]
    Week1,
    #[serde(alias = "1M")]
    Month1,
    #[serde(alias = "3M")]
    Month3,
}

impl Display for Interval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Interval::Minute1 => "1m",
                Interval::Minute3 => "3m",
                Interval::Minute5 => "5m",
                Interval::Minute15 => "15m",
                Interval::Minute30 => "30m",
                Interval::Hour1 => "1h",
                Interval::Hour2 => "2h",
                Interval::Hour4 => "4h",
                Interval::Hour6 => "6h",
                Interval::Hour8 => "8h",
                Interval::Hour12 => "12h",
                Interval::Day1 => "1d",
                Interval::Day3 => "3d",
                Interval::Week1 => "1w",
                Interval::Month1 => "1M",
                Interval::Month3 => "3M",
            }
        )
    }
}

/// Metadata generated from a collection of Barter [`Subscription`]s. This includes the exchange
/// specific subscription payloads that are sent to the exchange.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct SubscriptionMeta {
    /// `HashMap` containing the mapping between an incoming exchange message's [`SubscriptionId`],
    /// and a Barter [`Subscription`]. Used to identify the original [`Subscription`] associated
    /// with a received message.
    pub ids: SubscriptionIds,
    /// Number of [`Subscription`] responses expected from the exchange. Used to validate all
    /// [`Subscription`] were accepted.
    pub expected_responses: usize,
    /// Collection of [`WsMessage`]s containing exchange specific subscription payloads to be sent.
    pub subscriptions: Vec<WsMessage>,
}

/// Convenient type alias for a `HashMap` containing the mapping between an incoming exchange
/// message's [`SubscriptionId`], and a Barter [`Subscription`]. Used to identify the original
/// [`Subscription`] associated with a received message.
#[derive(Clone, Eq, PartialEq, Debug, Serialize)]
pub struct SubscriptionIds(pub HashMap<SubscriptionId, Subscription>);

impl Deref for SubscriptionIds {
    type Target = HashMap<SubscriptionId, Subscription>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SubscriptionIds {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'de> Deserialize<'de> for SubscriptionIds {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        HashMap::deserialize(deserializer).map(SubscriptionIds)
    }
}

/// Todo:
pub fn de_floats<'de, D>(deserializer: D) -> Result<f64, D::Error>
    where D: Deserializer<'de>, {
    let num_str: String = Deserialize::deserialize(deserializer)?;
    num_str.parse().map_err(|_| D::Error::custom("Float parsing error"))
}

impl SubscriptionIds {
    /// Find the [`Instrument`] associated with the provided [`SubscriptionId`] reference.
    pub fn find_instrument(&self, id: &SubscriptionId) -> Result<Instrument, SocketError> {
        self.get(id)
            .map(|subscription| subscription.instrument.clone())
            .ok_or_else(|| SocketError::Unidentifiable(id.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::Error;

    #[test]
    fn test_deserialise_subscription() {
        struct TestCase {
            input: &'static str,
            expected: Result<Subscription, serde_json::Error>,
        }

        let cases = vec![
            TestCase {
                // TC0: Valid Binance btc_usd Spot Trade Subscription
                input: r##"{"exchange": "binance", "base": "btc", "quote": "usd", "instrument_type": "spot", "type": "trade"}"##,
                expected: Ok(Subscription {
                    exchange: ExchangeId::Binance,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: SubKind::Trade,
                }),
            },
            TestCase {
                // TC1: Valid BinanceFuturesUsd btc_usd FuturePerpetual Trade Subscription
                input: r##"{"exchange": "binance_futures_usd", "base": "btc", "quote": "usd", "instrument_type": "future_perpetual", "type": "trade"}"##,
                expected: Ok(Subscription {
                    exchange: ExchangeId::BinanceFuturesUsd,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::FuturePerpetual)),
                    kind: SubKind::Trade,
                }),
            },
            TestCase {
                // TC2: Valid Binance btc_usd Spot Candle("5m") Subscription
                input: r##"{"exchange": "binance", "base": "btc", "quote": "usd", "instrument_type": "spot", "type": { "candle": "5m"}}"##,
                expected: Ok(Subscription {
                    exchange: ExchangeId::Binance,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: SubKind::Candle(Interval::Minute5),
                }),
            },
            TestCase {
                // TC3: Valid BinanceFuturesUsd btc_usd FuturePerpetual Candle("5m") Subscription
                input: r##"{"exchange": "binance_futures_usd", "base": "btc", "quote": "usd", "instrument_type": "future_perpetual", "type": { "candle": "5m"}}"##,
                expected: Ok(Subscription {
                    exchange: ExchangeId::BinanceFuturesUsd,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::FuturePerpetual)),
                    kind: SubKind::Candle(Interval::Minute5),
                }),
            },
            TestCase {
                // TC4: Valid Binance btc_usd Spot OrderBookL2 Subscription
                input: r##"{"exchange": "binance", "base": "btc", "quote": "usd", "instrument_type": "spot", "type": "order_book_l2"}"##,
                expected: Ok(Subscription {
                    exchange: ExchangeId::Binance,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: SubKind::OrderBookL2,
                }),
            },
            TestCase {
                // TC5: Valid BinanceFuturesUsd btc_usd FuturePerpetual OrderBookL2 Subscription
                input: r##"{"exchange": "binance_futures_usd", "base": "btc", "quote": "usd", "instrument_type": "future_perpetual", "type": "order_book_l2"}"##,
                expected: Ok(Subscription {
                    exchange: ExchangeId::BinanceFuturesUsd,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::FuturePerpetual)),
                    kind: SubKind::OrderBookL2,
                }),
            },
            TestCase {
                // TC6: Invalid Subscription w/ unknown exchange
                input: r##"{"exchange": "unknown", "base": "btc", "quote": "usd", "instrument_type": "future_perpetual", "type": "order_book_delta"}"##,
                expected: Err(serde_json::Error::custom("")),
            },
            TestCase {
                // TC7: Invalid Subscription w/ unknown SubKind
                input: r##"{"exchange": "binance_futures_usd", "base": "btc", "quote": "usd", "instrument_type": "future_perpetual", "type": "unknown"}"##,
                expected: Err(serde_json::Error::custom("")),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<Subscription>(test.input);

            match (actual, test.expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "TC{} failed", index)
                }
                (Err(_), Err(_)) => {
                    // Test passed
                }
                (actual, expected) => {
                    // Test failed
                    panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                }
            }
        }
    }

    #[test]
    fn test_subscription_validate() {
        struct TestCase {
            input: Subscription,
            expected: Result<Subscription, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: Valid Subscription w/ Binance Spot
                input: Subscription {
                    exchange: ExchangeId::Binance,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: SubKind::Trade,
                },
                expected: Ok(Subscription {
                    exchange: ExchangeId::Binance,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: SubKind::Trade,
                }),
            },
            TestCase {
                // TC1: Invalid Subscription w/ Binance FuturePerpetual
                input: Subscription {
                    exchange: ExchangeId::Binance,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::FuturePerpetual)),
                    kind: SubKind::Trade,
                },
                expected: Err(SocketError::Subscribe("".to_string())),
            },
            TestCase {
                // TC2: Valid Subscription w/ BinanceFuturesUsd FuturePerpetual
                input: Subscription {
                    exchange: ExchangeId::BinanceFuturesUsd,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::FuturePerpetual)),
                    kind: SubKind::Trade,
                },
                expected: Ok(Subscription {
                    exchange: ExchangeId::BinanceFuturesUsd,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::FuturePerpetual)),
                    kind: SubKind::Trade,
                }),
            },
            TestCase {
                // TC3: Invalid Subscription w/ BinanceFuturesUsd Spot
                input: Subscription {
                    exchange: ExchangeId::BinanceFuturesUsd,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: SubKind::Trade,
                },
                expected: Err(SocketError::Subscribe("".to_string())),
            },
            TestCase {
                // TC4: Valid Subscription w/ Ftx Spot
                input: Subscription {
                    exchange: ExchangeId::Ftx,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: SubKind::Trade,
                },
                expected: Ok(Subscription {
                    exchange: ExchangeId::Ftx,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: SubKind::Trade,
                }),
            },
            TestCase {
                // TC5: Valid Subscription w/ Ftx FuturePerpetual
                input: Subscription {
                    exchange: ExchangeId::Ftx,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::FuturePerpetual)),
                    kind: SubKind::Trade,
                },
                expected: Ok(Subscription {
                    exchange: ExchangeId::Ftx,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::FuturePerpetual)),
                    kind: SubKind::Trade,
                }),
            },
            TestCase {
                // TC4: Valid Subscription w/ Ftx Spot
                input: Subscription {
                    exchange: ExchangeId::Kraken,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: SubKind::Trade,
                },
                expected: Ok(Subscription {
                    exchange: ExchangeId::Kraken,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: SubKind::Trade,
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = test.input.validate();

            match (actual, test.expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, &expected, "TC{} failed", index)
                }
                (Err(_), Err(_)) => {
                    // Test passed
                }
                (actual, expected) => {
                    // Test failed
                    panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                }
            }
        }
    }

    #[test]
    fn test_subscription_ids_find_instrument() {
        // Initialise SubscriptionIds HashMap
        let ids = SubscriptionIds(HashMap::from_iter([(
            SubscriptionId::from("present"),
            Subscription::from((
                ExchangeId::Binance,
                "base",
                "quote",
                InstrumentKind::Spot,
                SubKind::Trade,
            )),
        )]));

        struct TestCase {
            input: SubscriptionId,
            expected: Result<Instrument, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: SubscriptionId (channel) is present in the HashMap
                input: SubscriptionId::from("present"),
                expected: Ok(Instrument::from(("base", "quote", InstrumentKind::Spot))),
            },
            TestCase {
                // TC1: SubscriptionId (channel) is not present in the HashMap
                input: SubscriptionId::from("not present"),
                expected: Err(SocketError::Unidentifiable(SubscriptionId::from(
                    "not present",
                ))),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = ids.find_instrument(&test.input);
            match (actual, test.expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "TC{} failed", index)
                }
                (Err(_), Err(_)) => {
                    // Test passed
                }
                (actual, expected) => {
                    // Test failed
                    panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                }
            }
        }
    }
}
