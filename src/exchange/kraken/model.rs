use super::Kraken;
use crate::{
    exchange::{datetime_utc_from_epoch_duration, extract_next, se_element_to_vector},
    model::{Candle, DataKind, Interval, PublicTrade, SubKind},
    ExchangeId, ExchangeTransformer, MarketEvent,
};
use barter_integration::{
    error::SocketError,
    model::{Exchange, Instrument, Side, SubscriptionId},
    protocol::websocket::WsMessage,
    Validator,
};
use chrono::{DateTime, Utc};
use serde::{de, Deserialize, Deserializer, Serialize};
use std::time::Duration;

/// [`Kraken`] compatible subscription messaged translated from a Barter
/// [`Subscription`](crate::Subscription).
///
/// eg/ KrakenSubscription {
///     "event": subscribe,
///     "pair": "XBT/USD"
///     "subscription": {
///         "channel": "ohlc",
///         "interval": "5",
///     }
/// }
/// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct KrakenSubscription {
    pub event: &'static str,
    #[serde(rename = "pair", serialize_with = "se_element_to_vector")]
    pub market: String,
    #[serde(rename = "subscription")]
    pub kind: KrakenSubKind,
}

impl From<&KrakenSubscription> for SubscriptionId {
    fn from(kraken_subscription: &KrakenSubscription) -> Self {
        match kraken_subscription.kind {
            KrakenSubKind::Trade { channel } => {
                // eg/ SubscriptionId::from("trade|XBT/USD")
                SubscriptionId::from(format!("{channel}|{}", kraken_subscription.market))
            }
            KrakenSubKind::Candle { channel, interval } => {
                // eg/ SubscriptionId::from("ohlc-5|XBT/USD"),
                SubscriptionId::from(format!(
                    "{channel}-{interval}|{}",
                    kraken_subscription.market
                ))
            }
        }
    }
}

impl TryFrom<&KrakenSubscription> for WsMessage {
    type Error = SocketError;

    fn try_from(kraken_sub: &KrakenSubscription) -> Result<Self, Self::Error> {
        serde_json::to_string(&kraken_sub)
            .map(WsMessage::text)
            .map_err(|error| SocketError::Serde {
                error,
                payload: format!("{kraken_sub:?}"),
            })
    }
}

impl KrakenSubscription {
    const EVENT: &'static str = "subscribe";

    /// Construct a new [`KrakenSubscription`] from the provided market identifier (eg/ "XBT/USD"),
    /// and [`KrakenSubKind`].
    pub fn new(market: String, kind: KrakenSubKind) -> Self {
        Self {
            event: Self::EVENT,
            market,
            kind,
        }
    }
}

/// Possible [`KrakenSubscription`] variants.
///
/// eg/ KrakenSubKind::Candle {
///         "channel": "ohlc",
///         "interval": "5",
/// }
/// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
#[serde(untagged)]
pub enum KrakenSubKind {
    Trade {
        #[serde(rename = "name")]
        channel: &'static str,
    },
    Candle {
        #[serde(rename = "name")]
        channel: &'static str,
        interval: u32,
    },
}

impl KrakenSubKind {
    const TRADE: &'static str = "trade";
    const CANDLE: &'static str = "ohlc";
}

impl TryFrom<&SubKind> for KrakenSubKind {
    type Error = SocketError;

    fn try_from(kind: &SubKind) -> Result<Self, Self::Error> {
        match kind {
            SubKind::Trade => Ok(KrakenSubKind::Trade {
                channel: KrakenSubKind::TRADE,
            }),
            SubKind::Candle(interval) => Ok(KrakenSubKind::Candle {
                channel: KrakenSubKind::CANDLE,
                interval: u32::from(KrakenInterval::try_from(interval)?),
            }),
            other => Err(SocketError::Unsupported {
                entity: Kraken::EXCHANGE.as_str(),
                item: other.to_string(),
            }),
        }
    }
}

/// Kraken time interval used for specifying the interval of a [`KrakenSubKind::Candle`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum KrakenInterval {
    Minute1,
    Minute5,
    Minute15,
    Minute30,
    Hour1,
    Hour4,
    Hour12,
    Day1,
    Week1,
}

impl TryFrom<&Interval> for KrakenInterval {
    type Error = SocketError;

    fn try_from(interval: &Interval) -> Result<Self, Self::Error> {
        match interval {
            Interval::Minute1 => Ok(KrakenInterval::Minute1),
            Interval::Minute5 => Ok(KrakenInterval::Minute5),
            Interval::Minute15 => Ok(KrakenInterval::Minute15),
            Interval::Minute30 => Ok(KrakenInterval::Minute30),
            Interval::Hour1 => Ok(KrakenInterval::Hour1),
            Interval::Hour4 => Ok(KrakenInterval::Hour4),
            Interval::Hour12 => Ok(KrakenInterval::Hour12),
            Interval::Day1 => Ok(KrakenInterval::Day1),
            Interval::Week1 => Ok(KrakenInterval::Week1),
            other => Err(SocketError::Unsupported {
                entity: Kraken::EXCHANGE.as_str(),
                item: other.to_string(),
            }),
        }
    }
}

impl From<KrakenInterval> for u32 {
    fn from(interval: KrakenInterval) -> Self {
        match interval {
            KrakenInterval::Minute1 => 1,
            KrakenInterval::Minute5 => 5,
            KrakenInterval::Minute15 => 15,
            KrakenInterval::Minute30 => 30,
            KrakenInterval::Hour1 => 60,
            KrakenInterval::Hour4 => 240,
            KrakenInterval::Hour12 => 1440,
            KrakenInterval::Day1 => 10080,
            KrakenInterval::Week1 => 21600,
        }
    }
}

/// `Kraken` message received in response to WebSocket subscription requests.
///
/// eg/ KrakenSubResponse::Subscribed {
///     "channelID":337,
///     "channelName":"trade",
///     "event":"subscriptionStatus",
///     "pair":"XBT/USD",
///     "status":"subscribed",
///     "subscription":{"name":"trade"}
/// }
/// eg/ KrakenSubResponse::Error {
///     "errorMessage":"Subscription name invalid",
///     "event":"subscriptionStatus",
///     "pair":"ETH/USD",
///     "status":"error",
///     "subscription":{"name":"trades"}
/// }
///
/// See docs: <https://docs.kraken.com/websockets/#message-subscriptionStatus>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum KrakenSubResponse {
    Subscribed {
        #[serde(alias = "channelID")]
        channel_id: u64,
        #[serde(alias = "channelName")]
        channel_name: String,
        pair: String,
    },
    Error(KrakenError),
}

/// `Kraken` generic error message String received over the WebSocket.
///
/// Note that since the [`KrakenError`] is only made up of a renamed message String field, it can
/// be used flexible as a [`KrakenSubResponse::Error`](KrakenSubResponse) or as a generic error
/// received over the WebSocket while subscriptions are active.
///
/// See docs generic: <https://docs.kraken.com/websockets/#errortypes>
/// See docs subscription failed: <https://docs.kraken.com/websockets/#message-subscriptionStatus>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct KrakenError {
    #[serde(alias = "errorMessage")]
    pub message: String,
}

impl Validator for KrakenSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self {
            KrakenSubResponse::Subscribed { .. } => Ok(self),
            KrakenSubResponse::Error(error) => Err(SocketError::Subscribe(format!(
                "received failure subscription response: {}",
                error.message
            ))),
        }
    }
}

/// `Kraken` message variants that can be received over [`WebSocket`](crate::WebSocket).
///
/// See docs: <https://docs.kraken.com/websockets/#overview>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum KrakenMessage {
    Trades(KrakenTrades),
    Candle(KrakenCandle),
    KrakenEvent(KrakenEvent),
}

/// Collection of [`KrakenTrade`] items with an associated [`SubscriptionId`] (eg/ "trade|XBT/USD").
///
/// See docs: <https://docs.kraken.com/websockets/#message-trade>
#[derive(Clone, PartialEq, PartialOrd, Debug, Serialize)]
pub struct KrakenTrades {
    pub subscription_id: SubscriptionId,
    pub trades: Vec<KrakenTrade>,
}

/// `Kraken` trade.
///
/// See docs: <https://docs.kraken.com/websockets/#message-trade>
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize)]
pub struct KrakenTrade {
    pub price: f64,
    pub quantity: f64,
    pub time: DateTime<Utc>,
    pub side: Side,
}

/// `Kraken` candle containing OHLCV [`KrakenCandleData`] with an associated [`SubscriptionId`]
/// (eg/ "candle|XBT/USD").
///
/// See docs: <https://docs.kraken.com/websockets/#message-ohlc>
#[derive(Clone, PartialEq, PartialOrd, Debug, Serialize)]
pub struct KrakenCandle {
    pub subscription_id: SubscriptionId,
    pub candle: KrakenCandleData,
}

/// `Kraken` OHLCV data.
///
/// See docs: <https://docs.kraken.com/websockets/#message-ohlc>
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize)]
pub struct KrakenCandleData {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub trade_count: u64,
}

/// `Kraken` messages received over the WebSocket which are not subscription data.
///
/// eg/ `Kraken` sends a `KrakenEvent::Heartbeat` if no subscription traffic has been sent
/// within the last second.
///
/// See docs: <https://docs.kraken.com/websockets/#message-heartbeat>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum KrakenEvent {
    Heartbeat,
    Error(KrakenError),
}

impl From<(ExchangeId, Instrument, KrakenTrade)> for MarketEvent {
    fn from((exchange_id, instrument, trade): (ExchangeId, Instrument, KrakenTrade)) -> Self {
        // kraken trades do not come with a unique identifier, so generated a custom one
        let custom_trade_id = format!(
            "{}_{:?}_{}_{}",
            trade.time, trade.side, trade.price, trade.quantity
        );

        Self {
            exchange_time: trade.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::Trade(PublicTrade {
                id: custom_trade_id,
                price: trade.price,
                quantity: trade.quantity,
                side: trade.side,
            }),
        }
    }
}

impl From<(ExchangeId, Instrument, KrakenCandleData)> for MarketEvent {
    fn from((exchange_id, instrument, candle): (ExchangeId, Instrument, KrakenCandleData)) -> Self {
        Self {
            exchange_time: candle.end_time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::Candle(Candle {
                start_time: candle.start_time,
                end_time: candle.end_time,
                open: candle.open,
                high: candle.high,
                low: candle.low,
                close: candle.close,
                volume: candle.volume,
                trade_count: candle.trade_count,
            }),
        }
    }
}

impl<'de> Deserialize<'de> for KrakenTrades {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SeqVisitor;

        impl<'de> de::Visitor<'de> for SeqVisitor {
            type Value = KrakenTrades;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("KrakenTrades struct from the Kraken WebSocket API")
            }

            fn visit_seq<SeqAccessor>(
                self,
                mut seq: SeqAccessor,
            ) -> Result<Self::Value, SeqAccessor::Error>
            where
                SeqAccessor: de::SeqAccess<'de>,
            {
                // KrakenTrades Sequence Format:
                // [channelID, [[price, volume, time, side, orderType, misc]], channelName, pair]
                // <https://docs.kraken.com/websockets/#message-trade>

                // Extract deprecated channelID & ignore
                let _: de::IgnoredAny = extract_next(&mut seq, "channelID")?;

                // Extract Vec<KrakenTrade>
                let trades = extract_next(&mut seq, "Vec<KrakenTrade>")?;

                // Extract channelName (eg/ "trade") & ignore
                let _: de::IgnoredAny = extract_next(&mut seq, "channelName")?;

                // Extract pair (eg/ "XBT/USD") & map to SubscriptionId (ie/ "trade|{pair}")
                let subscription_id = extract_next::<SeqAccessor, String>(&mut seq, "pair")
                    .map(|pair| SubscriptionId::from(format!("trade|{pair}")))?;

                // Ignore any additional elements or SerDe will fail
                //  '--> Exchange may add fields without warning
                while seq.next_element::<de::IgnoredAny>()?.is_some() {}

                Ok(KrakenTrades {
                    subscription_id,
                    trades,
                })
            }
        }

        // Use Visitor implementation to deserialise the KrakenTrades
        deserializer.deserialize_seq(SeqVisitor)
    }
}

impl<'de> Deserialize<'de> for KrakenTrade {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SeqVisitor;

        impl<'de> de::Visitor<'de> for SeqVisitor {
            type Value = KrakenTrade;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("KrakenTrade struct from the Kraken WebSocket API")
            }

            fn visit_seq<SeqAccessor>(
                self,
                mut seq: SeqAccessor,
            ) -> Result<Self::Value, SeqAccessor::Error>
            where
                SeqAccessor: de::SeqAccess<'de>,
            {
                // KrakenTrade Sequence Format:
                // [price, volume, time, side, orderType, misc]
                // <https://docs.kraken.com/websockets/#message-trade>

                // Extract String price & parse to f64
                let price = extract_next::<SeqAccessor, String>(&mut seq, "price")?
                    .parse()
                    .map_err(de::Error::custom)?;

                // Extract String quantity & parse to f64
                let quantity = extract_next::<SeqAccessor, String>(&mut seq, "quantity")?
                    .parse()
                    .map_err(de::Error::custom)?;

                // Extract String price, parse to f64, map to DateTime<Utc>
                let time = extract_next::<SeqAccessor, String>(&mut seq, "time")?
                    .parse()
                    .map(|time| datetime_utc_from_epoch_duration(Duration::from_secs_f64(time)))
                    .map_err(de::Error::custom)?;

                // Extract Side
                let side: Side = extract_next(&mut seq, "side")?;

                // Ignore any additional elements or SerDe will fail
                //  '--> Exchange may add fields without warning
                while seq.next_element::<de::IgnoredAny>()?.is_some() {}

                Ok(KrakenTrade {
                    price,
                    quantity,
                    time,
                    side,
                })
            }
        }

        // Use Visitor implementation to deserialise the KrakenTrade
        deserializer.deserialize_seq(SeqVisitor)
    }
}

impl<'de> Deserialize<'de> for KrakenCandle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SeqVisitor;

        impl<'de> de::Visitor<'de> for SeqVisitor {
            type Value = KrakenCandle;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("KrakenCandle struct from the Kraken WebSocket API")
            }

            fn visit_seq<SeqAccessor>(
                self,
                mut seq: SeqAccessor,
            ) -> Result<Self::Value, SeqAccessor::Error>
            where
                SeqAccessor: de::SeqAccess<'de>,
            {
                // KrakenCandle Sequence Format:
                // [channelID, [time, end_time, open, high, low, close, vwap, volume, count], channelName, pair]
                // <https://docs.kraken.com/websockets/#message-ohlc>

                // Extract deprecated channelID & ignore
                let _: de::IgnoredAny = extract_next(&mut seq, "channelID")?;

                // Extract OHLCV KrakenCandleData from inner sequence
                let candle = extract_next(&mut seq, "KrakenCandleData")?;

                // Extract channelName (eg/ "ohlc-5")
                let channel: String = extract_next(&mut seq, "channelName")?;

                // Extract pair (eg/ "XBT/USD") & map to SubscriptionId (eg/ "ohlc-5|XBT/USD")
                let subscription_id = extract_next::<SeqAccessor, String>(&mut seq, "pair")
                    .map(|pair| SubscriptionId::from(format!("{channel}|{pair}")))?;

                // Ignore any additional elements or SerDe will fail
                //  '--> Exchange may add fields without warning
                while seq.next_element::<de::IgnoredAny>()?.is_some() {}

                Ok(KrakenCandle {
                    subscription_id,
                    candle,
                })
            }
        }

        // Use Visitor implementation to deserialise the KrakenCandle
        deserializer.deserialize_seq(SeqVisitor)
    }
}

impl<'de> Deserialize<'de> for KrakenCandleData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SeqVisitor;

        impl<'de> de::Visitor<'de> for SeqVisitor {
            type Value = KrakenCandleData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("KrakenCandleData struct from the Kraken WebSocket API")
            }

            fn visit_seq<SeqAccessor>(
                self,
                mut seq: SeqAccessor,
            ) -> Result<Self::Value, SeqAccessor::Error>
            where
                SeqAccessor: de::SeqAccess<'de>,
            {
                // KrakenCandleData Sequence Format:
                // [time, end_time, open, high, low, close, vwap, volume, count]
                // <https://docs.kraken.com/websockets/#message-ohlc>

                // Extract String candle start time, parse to f64, map to DateTime<Utc>
                let start_time = extract_next::<SeqAccessor, String>(&mut seq, "start_time")?
                    .parse()
                    .map(|time| datetime_utc_from_epoch_duration(Duration::from_secs_f64(time)))
                    .map_err(de::Error::custom)?;

                // Extract String candle end time, parse to f64, map to DateTime<Utc>
                let end_time = extract_next::<SeqAccessor, String>(&mut seq, "end_time")?
                    .parse()
                    .map(|time| datetime_utc_from_epoch_duration(Duration::from_secs_f64(time)))
                    .map_err(de::Error::custom)?;

                // Extract String open
                let open = extract_next::<SeqAccessor, String>(&mut seq, "open")?
                    .parse()
                    .map_err(de::Error::custom)?;

                // Extract String high
                let high = extract_next::<SeqAccessor, String>(&mut seq, "high")?
                    .parse()
                    .map_err(de::Error::custom)?;

                // Extract String low
                let low = extract_next::<SeqAccessor, String>(&mut seq, "low")?
                    .parse()
                    .map_err(de::Error::custom)?;

                // Extract String close
                let close = extract_next::<SeqAccessor, String>(&mut seq, "close")?
                    .parse()
                    .map_err(de::Error::custom)?;

                // Extract String vwap & ignore
                let _: de::IgnoredAny = extract_next(&mut seq, "vwap")?;

                // Extract String volume
                let volume = extract_next::<SeqAccessor, String>(&mut seq, "volume")?
                    .parse()
                    .map_err(de::Error::custom)?;

                // Extract u64 count
                let trade_count = extract_next(&mut seq, "count")?;

                // Ignore any additional elements or SerDe will fail
                //  '--> Exchange may add fields without warning
                while seq.next_element::<de::IgnoredAny>()?.is_some() {}

                Ok(KrakenCandleData {
                    start_time,
                    end_time,
                    open,
                    high,
                    low,
                    close,
                    volume,
                    trade_count,
                })
            }
        }

        // Use Visitor implementation to deserialise the KrakenCandleData
        deserializer.deserialize_seq(SeqVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::Error;

    #[test]
    fn test_try_from_sub_kind_for_kraken_sub_kind() {
        struct TestCase {
            input: SubKind,
            expected: Result<KrakenSubKind, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: Kraken supported SubKind::Trade
                input: SubKind::Trade,
                expected: Ok(KrakenSubKind::Trade {
                    channel: KrakenSubKind::TRADE,
                }),
            },
            TestCase {
                // TC1: Kraken supported SubKind::Candle w/ supported Interval::Minute5
                input: SubKind::Candle(Interval::Minute5),
                expected: Ok(KrakenSubKind::Candle {
                    channel: KrakenSubKind::CANDLE,
                    interval: 5,
                }),
            },
            TestCase {
                // TC2: Kraken supported SubKind::Candle w/ unsupported Interval::Month3
                input: SubKind::Candle(Interval::Month3),
                expected: Err(SocketError::Unsupported {
                    entity: "kraken",
                    item: "3M".to_string(),
                }),
            },
            TestCase {
                // TC3: Kraken unsupported SubKind::OrderBookL2
                input: SubKind::OrderBookL2,
                expected: Err(SocketError::Unsupported {
                    entity: "kraken",
                    item: "order_books_l2".to_string(),
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = KrakenSubKind::try_from(&test.input);
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
    fn test_try_from_interval_for_kraken_interval() {
        struct TestCase {
            input: Interval,
            expected: Result<KrakenInterval, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: Kraken supported Interval::Minute5
                input: Interval::Minute5,
                expected: Ok(KrakenInterval::Minute5),
            },
            TestCase {
                // TC1: Kraken unsupported Interval::Hour6
                input: Interval::Hour6,
                expected: Err(SocketError::Unsupported {
                    entity: "kraken",
                    item: "6h".to_string(),
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = KrakenInterval::try_from(&test.input);
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
    fn test_deserialise_kraken_subscription_response() {
        struct TestCase {
            input: &'static str,
            expected: Result<KrakenSubResponse, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is Subscribed
                input: r#"{"channelID":10001, "channelName":"trade", "event":"subscriptionStatus", "pair":"XBT/EUR", "status":"subscribed", "subscription":{"name":"ticker"}}"#,
                expected: Ok(KrakenSubResponse::Subscribed {
                    channel_id: 10001,
                    channel_name: "trade".to_owned(),
                    pair: "XBT/EUR".to_owned(),
                }),
            },
            TestCase {
                // TC1: input response is Error
                input: r#"{"errorMessage":"Subscription depth not supported","event":"subscriptionStatus","pair":"XBT/USD","status":"error","subscription":{"depth":42,"name":"book"}}"#,
                expected: Ok(KrakenSubResponse::Error(KrakenError {
                    message: "Subscription depth not supported".to_string(),
                })),
            },
            TestCase {
                // TC2: input response is malformed gibberish
                input: r#"{"type": "gibberish", "help": "please"}"#,
                expected: Err(SocketError::Serde {
                    error: serde_json::Error::custom(""),
                    payload: "".to_owned(),
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<KrakenSubResponse>(test.input);
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
    fn test_validate_kraken_subscription_response() {
        struct TestCase {
            input_response: KrakenSubResponse,
            is_valid: bool,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is Subscribed
                input_response: KrakenSubResponse::Subscribed {
                    channel_name: "trade".to_owned(),
                    pair: "XBT/EUR".to_owned(),
                    channel_id: 10001,
                },
                is_valid: true,
            },
            TestCase {
                // TC1: input response is Error
                input_response: KrakenSubResponse::Error(KrakenError {
                    message: "error message".to_string(),
                }),
                is_valid: false,
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = test.input_response.validate().is_ok();
            assert_eq!(actual, test.is_valid, "TestCase {} failed", index);
        }
    }

    #[test]
    fn test_deserialise_kraken_messages() {
        struct TestCase {
            input: &'static str,
            expected: Result<KrakenMessage, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: valid KrakenMessage Spot trades
                input: r#"[337,[["20180.30000","0.00010000","1661978265.280067","s","l",""],["20180.20000","0.00012340","1661978265.281568","s","l",""]],"trade","XBT/USD"]"#,
                expected: Ok(KrakenMessage::Trades(KrakenTrades {
                    trades: vec![
                        KrakenTrade {
                            price: 20180.30000,
                            quantity: 0.00010000,
                            time: datetime_utc_from_epoch_duration(Duration::from_secs_f64(
                                1661978265.280067,
                            )),
                            side: Side::Sell,
                        },
                        KrakenTrade {
                            price: 20180.20000,
                            quantity: 0.00012340,
                            time: datetime_utc_from_epoch_duration(Duration::from_secs_f64(
                                1661978265.281568,
                            )),
                            side: Side::Sell,
                        },
                    ],
                    subscription_id: SubscriptionId::from("trade|XBT/USD"),
                })),
            },
            TestCase {
                // TC1: valid KrakenMessage Spot trades w/ unexpected extra array fields
                input: r#"[337,[["3.30000","1000.00010000","1661978265.280067","b","l","", "", "", ""]],"trade","XBT/USD", "", ""]"#,
                expected: Ok(KrakenMessage::Trades(KrakenTrades {
                    trades: vec![KrakenTrade {
                        price: 3.30000,
                        quantity: 1000.00010000,
                        time: datetime_utc_from_epoch_duration(Duration::from_secs_f64(
                            1661978265.280067,
                        )),
                        side: Side::Buy,
                    }],
                    subscription_id: SubscriptionId::from("trade|XBT/USD"),
                })),
            },
            TestCase {
                // TC2: valid KrakenMessage Spot candles w/ unexpected extra array fields in outer & inner array
                input: r#"[42, ["1542057314.748456","1542057360.435743","7000.70000","7000.70000","1000.60000","3586.60000","3586.68894","0.03373000",50000, ""], "ohlc-5", "XBT/USD", ""]"#,
                expected: Ok(KrakenMessage::Candle(KrakenCandle {
                    subscription_id: SubscriptionId::from("ohlc-5|XBT/USD"),
                    candle: KrakenCandleData {
                        start_time: datetime_utc_from_epoch_duration(Duration::from_secs_f64(
                            1542057314.748456,
                        )),
                        end_time: datetime_utc_from_epoch_duration(Duration::from_secs_f64(
                            1542057360.435743,
                        )),
                        open: 7000.70000,
                        high: 7000.70000,
                        low: 1000.60000,
                        close: 3586.60000,
                        volume: 0.03373000,
                        trade_count: 50000,
                    },
                })),
            },
            TestCase {
                // TC3: valid KrakenMessage Heartbeat
                input: r#"{"event": "heartbeat"}"#,
                expected: Ok(KrakenMessage::KrakenEvent(KrakenEvent::Heartbeat)),
            },
            TestCase {
                // TC4: valid KrakenMessage Error
                input: r#"{"errorMessage": "Malformed request", "event": "error"}"#,
                expected: Ok(KrakenMessage::KrakenEvent(KrakenEvent::Error(
                    KrakenError {
                        message: "Malformed request".to_string(),
                    },
                ))),
            },
            TestCase {
                // TC5: invalid KrakenMessage gibberish
                input: r#"{"type": "gibberish", "help": "please"}"#,
                expected: Err(SocketError::Serde {
                    error: serde_json::Error::custom(""),
                    payload: "".to_owned(),
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<KrakenMessage>(test.input);
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
    fn test_deserialise_kraken_trade() {
        struct TestCase {
            input: &'static str,
            expected: Result<KrakenTrade, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: KrakenTrade is valid
                input: r#"["20180.30000","0.00010000","1661978265.280067","s","l",""]"#,
                expected: Ok(KrakenTrade {
                    price: 20180.30000,
                    quantity: 0.00010000,
                    time: datetime_utc_from_epoch_duration(Duration::from_secs_f64(
                        1661978265.280067,
                    )),
                    side: Side::Sell,
                }),
            },
            TestCase {
                // TC1: KrakenTrade is valid w/ unexpected additional sequence elements
                input: r#"["1000.0","1000.00010000","1661978265.0","b","l","", "", "", "", ""]"#,
                expected: Ok(KrakenTrade {
                    price: 1000.0,
                    quantity: 1000.00010000,
                    time: datetime_utc_from_epoch_duration(Duration::from_secs_f64(1661978265.0)),
                    side: Side::Buy,
                }),
            },
            TestCase {
                // TC2: KrakenTrade is invalid w/ non-string price
                input: r#"[1000.0,"1000.00010000","1661978265.0","b","l",""]"#,
                expected: Err(SocketError::Serde {
                    error: serde_json::Error::custom(""),
                    payload: "".to_owned(),
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<KrakenTrade>(test.input);
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
    fn test_deserialise_kraken_candle_data() {
        struct TestCase {
            input: &'static str,
            expected: Result<KrakenCandleData, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: KrakenCandleData is valid
                input: r#"["1542057314.748456","1542057360.435743","3586.70000","3586.70000","3586.60000","3586.60000","3586.68894","0.03373000",2]"#,
                expected: Ok(KrakenCandleData {
                    start_time: datetime_utc_from_epoch_duration(Duration::from_secs_f64(
                        1542057314.748456,
                    )),
                    end_time: datetime_utc_from_epoch_duration(Duration::from_secs_f64(
                        1542057360.435743,
                    )),
                    open: 3586.70000,
                    high: 3586.70000,
                    low: 3586.60000,
                    close: 3586.60000,
                    volume: 0.03373000,
                    trade_count: 2,
                }),
            },
            TestCase {
                // TC1: KrakenCandleData is valid w/ unexpected additional sequence elements
                input: r#"["1542057314.1","1542057360.2","7000.70000","7000.70000","1000.60000","3586.60000","3586.68894","0.03373000",50000, "", ""]"#,
                expected: Ok(KrakenCandleData {
                    start_time: datetime_utc_from_epoch_duration(Duration::from_secs_f64(
                        1542057314.1,
                    )),
                    end_time: datetime_utc_from_epoch_duration(Duration::from_secs_f64(
                        1542057360.2,
                    )),
                    open: 7000.70000,
                    high: 7000.70000,
                    low: 1000.60000,
                    close: 3586.60000,
                    volume: 0.03373000,
                    trade_count: 50000,
                }),
            },
            TestCase {
                // TC2: KrakenCandleData is invalid w/ f64 trade_count
                input: r#"["1542057314.748456","1542057360.435743","3586.70000","3586.70000","3586.60000","3586.60000","3586.68894","0.03373000",2.0]"#,
                expected: Err(SocketError::Serde {
                    error: serde_json::Error::custom(""),
                    payload: "".to_owned(),
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<KrakenCandleData>(test.input);
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
