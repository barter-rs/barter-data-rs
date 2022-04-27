use barter_integration::{Instrument, InstrumentKind, Sequence, Symbol};
use std::{
    fmt::{Debug, Display, Formatter},
    ops::Deref,
};
use serde::{de, Deserialize, Deserializer, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// Normalised Barter `MarketEvent` containing a [`MarketData`] variant, and the associated
/// `timestamp` and `sequence` number metadata.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct MarketEvent {
    pub sequence: Sequence,
    pub timestamp: DateTime<Utc>,
    pub data: MarketData,
}

impl MarketEvent {
    pub fn new(sequence: Sequence, data: MarketData) -> Self {
        Self {
            sequence,
            timestamp: Utc::now(),
            data
        }
    }
}

/// Possible public market data types.
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
    pub price: Decimal,
    pub quantity: Decimal,
    pub direction: Direction,
}

/// Direction of a [`Trade`]. Todo:
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum Direction {
    Buy,
    Sell
}

/// Barter [`Subscription`] used to subscribe to a market [`StreamKind`] for a particular
/// [`Instrument`].
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Subscription {
    pub instrument: Instrument,
    pub kind: StreamKind,
}

impl Debug for Subscription {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.kind, self.instrument)
    }
}

impl Display for Subscription {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<I> From<(I, StreamKind)> for Subscription
where
    I: Into<Instrument>
{
    fn from((instrument, kind): (I, StreamKind)) -> Self {
        Self {
            instrument: instrument.into(),
            kind
        }
    }
}

impl<S> From<(S, S, InstrumentKind, StreamKind)> for Subscription
where
    S: Into<Symbol>
{
    fn from((base, quote, instrument, stream): (S, S, InstrumentKind, StreamKind)) -> Self {
        Self {
            instrument: Instrument::from((base, quote, instrument)),
            kind: stream
        }
    }
}

impl Subscription {
    /// Constructs a new [`Subscription`] using the provided configuration.
    pub fn new<I>(instrument: I, kind: StreamKind) -> Self
    where
        I: Into<Instrument>
    {
        Self {
            instrument: instrument.into(),
            kind
        }
    }
}

/// Possible Stream types a [`Subscription`] is associated with.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamKind {
    Trades,
    Candles(Interval),
    Klines(Interval),
    OrderBookDeltas,
    OrderBooks,
}

impl Display for StreamKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            StreamKind::Trades => "trades".to_owned(),
            StreamKind::Candles(interval) => format!("candles_{}", interval),
            StreamKind::Klines(interval) => format!("klines_{}", interval),
            StreamKind::OrderBookDeltas => "order_book_deltas".to_owned(),
            StreamKind::OrderBooks => "order_books".to_owned()
        })

    }
}

/// Barter new type representing a time interval `String` identifier.
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

impl<'de> de::Deserialize<'de> for Interval {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Interval::new)
    }
}

impl<S> From<S> for Interval
where
    S: Into<String>
{
    fn from(input: S) -> Self {
        Self(input.into())
    }
}

impl Interval {
    /// Construct an [`Interval`] new type using the provided `Into<Interval>` value.
    pub fn new<S>(input: S) -> Self
    where
        S: Into<Interval>
    {
        input.into()
    }
}

/// Todo:
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct StreamMeta {
    pub sequence: Sequence,
    pub subscription: Subscription,
}

impl StreamMeta {
    /// Construct a new [`StreamMeta`] using the [`Subscription`] provided.
    pub fn new(subscription: Subscription) -> Self {
        Self {
            sequence: Sequence(0),
            subscription
        }
    }
}

/// Todo:
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub struct StreamId(pub String);

impl Debug for StreamId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for StreamId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for StreamId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Deref for StreamId {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for StreamId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        String::deserialize(deserializer).map(StreamId)
    }
}

impl<S> From<S> for StreamId
where
    S: Into<String>
{
    fn from(input: S) -> Self {
        Self(input.into())
    }
}