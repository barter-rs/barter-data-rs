use crate::{ExchangeId, Validator};
use barter_integration::{
    error::SocketError,
    model::{Exchange, Instrument, InstrumentKind, Market, Side, SubscriptionId, Symbol},
    protocol::websocket::WsMessage,
    Event,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::HashMap,
    fmt::{Debug, Display, Formatter},
    ops::{Deref, DerefMut},
};

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
            _ => Err(SocketError::Subscribe(format!(
                "{} ExchangeTransformer does not support InstrumentKinds of provided Subscriptions",
                self.exchange
            ))),
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
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubKind {
    Trade,
    Candle(Interval),
    Kline(Interval),
    OrderBook,
    OrderBookDelta,
}

impl Display for SubKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SubKind::Trade => "trades".to_owned(),
                SubKind::Candle(interval) => format!("candles_{}", interval),
                SubKind::Kline(interval) => format!("klines_{}", interval),
                SubKind::OrderBookDelta => "order_book_deltas".to_owned(),
                SubKind::OrderBook => "order_books".to_owned(),
            }
        )
    }
}

/// Barter new type representing a time interval as a `String` identifier.
///
/// eg/ "1m", "1h", "12h", "1d", "1w", "1M", etc
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub struct Interval(pub String);

impl Debug for Interval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for Interval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Interval {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Interval {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Interval::new)
    }
}

impl<S> From<S> for Interval
where
    S: Into<String>,
{
    fn from(input: S) -> Self {
        Self(input.into())
    }
}

impl Interval {
    /// Construct an [`Interval`] new type using the provided `Into<Interval>` value.
    pub fn new<S>(input: S) -> Self
    where
        S: Into<Interval>,
    {
        input.into()
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

impl SubscriptionIds {
    /// Find the [`Instrument`] associated with the provided `Into<SubscriptionId>`.
    pub fn find_instrument<Id>(&self, id: Id) -> Result<Instrument, SocketError>
    where
        Id: Into<SubscriptionId>,
    {
        let subscription_id: SubscriptionId = id.into();
        self.get(&subscription_id)
            .map(|subscription| subscription.instrument.clone())
            .ok_or(SocketError::Unidentifiable(subscription_id))
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
                    kind: SubKind::Candle(Interval::from("5m")),
                }),
            },
            TestCase {
                // TC3: Valid BinanceFuturesUsd btc_usd FuturePerpetual Candle("5m") Subscription
                input: r##"{"exchange": "binance_futures_usd", "base": "btc", "quote": "usd", "instrument_type": "future_perpetual", "type": { "candle": "5m"}}"##,
                expected: Ok(Subscription {
                    exchange: ExchangeId::BinanceFuturesUsd,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::FuturePerpetual)),
                    kind: SubKind::Candle(Interval::from("5m")),
                }),
            },
            TestCase {
                // TC4: Valid Binance btc_usd Spot Kline("5m") Subscription
                input: r##"{"exchange": "binance", "base": "btc", "quote": "usd", "instrument_type": "spot", "type": { "kline": "5m"}}"##,
                expected: Ok(Subscription {
                    exchange: ExchangeId::Binance,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: SubKind::Kline(Interval::from("5m")),
                }),
            },
            TestCase {
                // TC5: Valid BinanceFuturesUsd btc_usd FuturePerpetual Kline("5m") Subscription
                input: r##"{"exchange": "binance_futures_usd", "base": "btc", "quote": "usd", "instrument_type": "future_perpetual", "type": { "kline": "5m"}}"##,
                expected: Ok(Subscription {
                    exchange: ExchangeId::BinanceFuturesUsd,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::FuturePerpetual)),
                    kind: SubKind::Kline(Interval::from("5m")),
                }),
            },
            TestCase {
                // TC6: Valid Binance btc_usd Spot OrderBook Subscription
                input: r##"{"exchange": "binance", "base": "btc", "quote": "usd", "instrument_type": "spot", "type": "order_book"}"##,
                expected: Ok(Subscription {
                    exchange: ExchangeId::Binance,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: SubKind::OrderBook,
                }),
            },
            TestCase {
                // TC7: Valid BinanceFuturesUsd btc_usd FuturePerpetual OrderBook Subscription
                input: r##"{"exchange": "binance_futures_usd", "base": "btc", "quote": "usd", "instrument_type": "future_perpetual", "type": "order_book"}"##,
                expected: Ok(Subscription {
                    exchange: ExchangeId::BinanceFuturesUsd,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::FuturePerpetual)),
                    kind: SubKind::OrderBook,
                }),
            },
            TestCase {
                // TC8: Valid Binance btc_usd Spot OrderBookDelta Subscription
                input: r##"{"exchange": "binance", "base": "btc", "quote": "usd", "instrument_type": "spot", "type": "order_book_delta"}"##,
                expected: Ok(Subscription {
                    exchange: ExchangeId::Binance,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: SubKind::OrderBookDelta,
                }),
            },
            TestCase {
                // TC9: Valid BinanceFuturesUsd btc_usd FuturePerpetual OrderBookDelta Subscription
                input: r##"{"exchange": "binance_futures_usd", "base": "btc", "quote": "usd", "instrument_type": "future_perpetual", "type": "order_book_delta"}"##,
                expected: Ok(Subscription {
                    exchange: ExchangeId::BinanceFuturesUsd,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::FuturePerpetual)),
                    kind: SubKind::OrderBookDelta,
                }),
            },
            TestCase {
                // TC10: Invalid Subscription w/ gibberish exchange
                input: r##"{"exchange": "gibberish", "base": "btc", "quote": "usd", "instrument_type": "future_perpetual", "type": "order_book_delta"}"##,
                expected: Err(serde_json::Error::custom("")),
            },
            TestCase {
                // TC10: Invalid Subscription w/ gibberish SubKind
                input: r##"{"exchange": "binance_futures_usd", "base": "btc", "quote": "usd", "instrument_type": "future_perpetual", "type": "gibberish"}"##,
                expected: Err(serde_json::Error::custom("")),
            },
            TestCase {
                // TC10: Invalid Subscription w/ gibberish
                input: r##"{ "gibberish": "shouldfail"}"##,
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
            let actual = ids.find_instrument(test.input);
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
