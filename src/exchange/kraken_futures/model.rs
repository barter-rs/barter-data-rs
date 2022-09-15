use super::KrakenFuturesUsd;
use crate::{
    exchange::datetime_utc_from_epoch_duration,
    model::{DataKind, PublicTrade, SubKind},
    ExchangeId, ExchangeTransformer, MarketEvent,
};
use barter_integration::{
    error::SocketError,
    model::{Exchange, Instrument, Side, SubscriptionId},
    protocol::websocket::WsMessage,
    Validator,
};
use chrono::Utc;
use serde::{Deserialize, Serialize, Serializer};
use std::time::Duration;

/// [`KrakenFuturesUsd`] compatible subscription messaged translated from a Barter
/// [`Subscription`](crate::Subscription).
///
/// eg/ KrakenFuturesUsdSubscription {
///     "event": subscribe,
///     "feed": "trade"
///     "product_ids": [
///         "PI_XBTUSD",
///     ]
/// }
/// See docs: <https://support.kraken.com/hc/en-us/articles/360022839771-Trade>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct KrakenFuturesUsdSubscription {
    pub event: &'static str,
    #[serde(rename = "feed")]
    pub kind: KrakenFuturesUsdSubKind,
    #[serde(rename = "product_ids", serialize_with = "str_to_vec_serialize")]
    pub pair: String,
}

/// Kraken pairs need to be serialized as vectors, even though we will only ever
/// have one pair per subscription
fn str_to_vec_serialize<S>(x: &str, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.collect_seq(vec![x])
}

impl TryFrom<&KrakenFuturesUsdSubscription> for WsMessage {
    type Error = SocketError;

    fn try_from(kraken_sub: &KrakenFuturesUsdSubscription) -> Result<Self, Self::Error> {
        serde_json::to_string(&kraken_sub)
            .map(WsMessage::text)
            .map_err(|error| SocketError::Serde {
                error,
                payload: format!("{kraken_sub:?}"),
            })
    }
}

impl KrakenFuturesUsdSubscription {
    const EVENT: &'static str = "subscribe";

    /// Construct a new [`KrakenFuturesUsdSubscription`] from the provided pair (eg/ "PI_XBTUSD")
    /// and [`KrakenFuturesUsdSubKind`].
    pub fn new(pair: String, kind: KrakenFuturesUsdSubKind) -> Self {
        Self {
            event: Self::EVENT,
            pair,
            kind,
        }
    }
}

impl From<&KrakenFuturesUsdSubscription> for SubscriptionId {
    fn from(kraken_subscription: &KrakenFuturesUsdSubscription) -> Self {
        match kraken_subscription.kind {
            KrakenFuturesUsdSubKind::Trade(_trade) => {
                // eg/ SubscriptionId::from("PI_XBTUSD")
                SubscriptionId::from(format!("{}", kraken_subscription.pair))
            }
        }
    }
}

/// Possible [`KrakenFuturesUsdSubscription`] variants.
///
/// eg/ KrakenFuturesUsdSubKind::Trade {
///         "feed": "trade",
/// }
/// See docs: <https://support.kraken.com/hc/en-us/articles/360022839771-Trade>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
#[serde(untagged)]
pub enum KrakenFuturesUsdSubKind {
    Trade(&'static str),
}

impl KrakenFuturesUsdSubKind {
    const TRADE: &'static str = "trade";
}

impl TryFrom<&SubKind> for KrakenFuturesUsdSubKind {
    type Error = SocketError;

    fn try_from(kind: &SubKind) -> Result<Self, Self::Error> {
        match kind {
            SubKind::Trade => Ok(KrakenFuturesUsdSubKind::Trade(
                KrakenFuturesUsdSubKind::TRADE,
            )),
            other => Err(SocketError::Unsupported {
                entity: KrakenFuturesUsd::EXCHANGE.as_str(),
                item: other.to_string(),
            }),
        }
    }
}

/// `KrakenFuturesUsd` message received in response to WebSocket subscription requests.
///
/// eg/ KrakenFuturesSubResponse::Subscribed {"event":"subscribed","feed":"trade","product_ids":["PI_XBTUSD",]}
/// eg/ KrakenFuturesSubResponse::Error {"event": "alert", "message" : "Bad request"}
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum KrakenFuturesUsdSubResponse {
    Subscribed {
        feed: String,
        product_ids: Vec<String>,
    },
    #[serde(rename = "alert")]
    Error { message: String },
}

impl Validator for KrakenFuturesUsdSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self {
            KrakenFuturesUsdSubResponse::Subscribed { .. } => Ok(self),
            KrakenFuturesUsdSubResponse::Error { message, .. } => Err(SocketError::Subscribe(
                format!("received failure subscription response: {}", message),
            )),
        }
    }
}

/// `KrakenFuturesUsd` Message variants that can be received over [`WebSocket`].
/// [`TradeSnapshot`] is sent once after a [`WebSocket`] connection has been made.
/// A [`TradeSnapshot`] consists of the most recent 100 trades for a given pair -
/// currently this message is being ignored, and only new trades are being processed.
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(tag = "feed")]
pub enum KrakenFuturesUsdMessage {
    #[serde(rename = "trade_snapshot")]
    TradeSnapshot {
        product_id: SubscriptionId,
        trades: Vec<KrakenFuturesUsdTrade>,
    },
    #[serde(rename = "trade")]
    Trade(KrakenFuturesUsdTrade),
}

/// `KrakenFutures` trade message.
#[derive(Clone, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
pub struct KrakenFuturesUsdTrade {
    pub product_id: SubscriptionId,
    pub uid: String,
    pub side: Side,
    pub seq: u64,
    pub time: u64,
    #[serde(rename = "qty")]
    pub quantity: f64,
    pub price: f64,
}

impl From<(ExchangeId, Instrument, KrakenFuturesUsdTrade)> for MarketEvent {
    fn from(
        (exchange_id, instrument, trade): (ExchangeId, Instrument, KrakenFuturesUsdTrade),
    ) -> Self {
        Self {
            // time is received as ms
            exchange_time: datetime_utc_from_epoch_duration(Duration::new(
                trade.time / 1000,
                (trade.time % 1000 * 1000000) as u32,
            )),
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::Trade(PublicTrade {
                id: trade.uid,
                price: trade.price,
                quantity: trade.quantity,
                side: trade.side,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::Error;

    #[test]
    fn test_deserialise_kraken_futures_subscription_response() {
        struct TestCase {
            input: &'static str,
            expected: Result<KrakenFuturesUsdSubResponse, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is Subscribed
                input: r#"{"event": "subscribed", "feed": "trade", "product_ids": ["PI_XBTUSD"]}"#,
                expected: Ok(KrakenFuturesUsdSubResponse::Subscribed {
                    feed: "trade".to_owned(),
                    product_ids: vec!["PI_XBTUSD".to_owned()],
                }),
            },
            TestCase {
                // TC1: input response is Error
                input: r#"{"event":"alert","message":"Bad request"}"#,
                expected: Ok(KrakenFuturesUsdSubResponse::Error {
                    message: "Bad request".to_owned(),
                }),
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
            let actual = serde_json::from_str::<KrakenFuturesUsdSubResponse>(test.input);
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
    fn test_deserialise_kraken_futures_message() {
        struct TestCase {
            input: &'static str,
            expected: Result<KrakenFuturesUsdMessage, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: invalid KrakenFuturesUsdMessage w/ unknown tag
                input: r#"{"type": "unknown", "sequence": 50,"product_id": "BTC-USD"}"#,
                expected: Err(SocketError::Serde {
                    error: serde_json::Error::custom(""),
                    payload: "".to_owned(),
                }),
            },
            TestCase {
                // TC1: valid KrakenFuturesUsd Future Perpetual trades
                input: r#"{
                    "feed": "trade","product_id": "PI_XBTUSD",
                    "uid": "45ee9737-1877-4682-bc68-e4ef818ef88a",
                    "side": "sell",
                    "type": "fill", "seq": 655507, "time": 1612269656839, "qty":9643 , "price" : 34891
                }"#,
                expected: Ok(KrakenFuturesUsdMessage::Trade(KrakenFuturesUsdTrade {
                    product_id: SubscriptionId::from("PI_XBTUSD"),
                    uid: "45ee9737-1877-4682-bc68-e4ef818ef88a".to_owned(),
                    price: 34891.0,
                    quantity: 9643.0,
                    seq: 655507,
                    side: Side::Sell,
                    time: 1612269656839,
                })),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<KrakenFuturesUsdMessage>(test.input);
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
