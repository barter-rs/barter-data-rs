use super::Coinbase;
use crate::{
    exchange::de_str,
    model::{DataKind, PublicTrade},
    ExchangeId, MarketEvent, Validator,
};
use barter_integration::{
    error::SocketError,
    model::{Exchange, Instrument, Side, SubscriptionId},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// [`Coinbase`] message received in response to WebSocket subscription requests.
///
/// eg/ CoinbaseResponse::Subscribed {"type": "subscriptions", "channels": [{"name": "matches", "product_ids": ["BTC-USD"]}]}
/// eg/ CoinbaseResponse::Error {"type": "error", "message": "error message", "reason": "reason"}
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CoinbaseSubResponse {
    #[serde(alias = "subscriptions")]
    Subscribed {
        channels: Vec<CoinbaseChannels>,
    },
    Error {
        reason: String,
    },
}

/// Communicates the [`Coinbase`] product_ids (eg/ "ETH-USD") associated with a successful channel
/// (eg/ "matches") subscription.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct CoinbaseChannels {
    #[serde(alias = "name")]
    pub channel: String,
    pub product_ids: Vec<String>,
}

impl Validator for CoinbaseSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self {
            CoinbaseSubResponse::Subscribed { .. } => Ok(self),
            CoinbaseSubResponse::Error { reason } => Err(SocketError::Subscribe(format!(
                "received failure subscription response: {}",
                reason
            ))),
        }
    }
}

/// [`Coinbase`] message variants that can be received over [`WebSocket`].
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels>
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum CoinbaseMessage {
    #[serde(alias = "match", alias = "last_match")]
    Trade(CoinbaseTrade),
}

/// [`Coinbase`] trade message.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#match>
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct CoinbaseTrade {
    #[serde(alias = "product_id", deserialize_with = "de_trade_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(alias = "trade_id")]
    pub id: u64,
    pub time: DateTime<Utc>,
    #[serde(alias = "size", deserialize_with = "de_str")]
    pub quantity: f64,
    #[serde(deserialize_with = "de_str")]
    pub price: f64,
    pub side: Side,
}

impl From<(ExchangeId, Instrument, CoinbaseTrade)> for MarketEvent {
    fn from((exchange_id, instrument, trade): (ExchangeId, Instrument, CoinbaseTrade)) -> Self {
        Self {
            exchange_time: trade.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::Trade(PublicTrade {
                id: trade.id.to_string(),
                price: trade.price,
                quantity: trade.quantity,
                side: trade.side,
            }),
        }
    }
}

/// Deserialize a [`CoinbaseTrade`] "product_id" (eg/ "BTC-USD") as the associated [`SubscriptionId`]
/// (eg/ SubscriptionId("matches|BTC-USD").
pub fn de_trade_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    serde::de::Deserialize::deserialize(deserializer)
        .map(|product_id| Coinbase::subscription_id(Coinbase::CHANNEL_TRADES, product_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;
    use serde::de::Error;
    use std::str::FromStr;

    #[test]
    fn test_deserialise_coinbase_subscription_response() {
        struct TestCase {
            input: &'static str,
            expected: Result<CoinbaseSubResponse, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is Subscribed
                input: r#"{"type": "subscriptions", "channels": [{"name": "matches", "product_ids": ["BTC-USD"]}]}"#,
                expected: Ok(CoinbaseSubResponse::Subscribed {
                    channels: vec![CoinbaseChannels {
                        channel: "matches".to_owned(),
                        product_ids: vec!["BTC-USD".to_owned()],
                    }],
                }),
            },
            TestCase {
                // TC1: input response is Error
                input: r#"{"type":"error","message":"Failed to subscribe","reason":"matches is not a valid product"}"#,
                expected: Ok(CoinbaseSubResponse::Error {
                    reason: "matches is not a valid product".to_owned(),
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
            let actual = serde_json::from_str::<CoinbaseSubResponse>(test.input);
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
    fn test_validate_coinbase_subscription_response() {
        struct TestCase {
            input_response: CoinbaseSubResponse,
            is_valid: bool,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is Subscribed
                input_response: CoinbaseSubResponse::Subscribed {
                    channels: vec![CoinbaseChannels {
                        channel: "".to_owned(),
                        product_ids: vec!["".to_owned()],
                    }],
                },
                is_valid: true,
            },
            TestCase {
                // TC1: input response is Error
                input_response: CoinbaseSubResponse::Error {
                    reason: "error message".to_owned(),
                },
                is_valid: false,
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = test.input_response.validate().is_ok();
            assert_eq!(actual, test.is_valid, "TestCase {} failed", index);
        }
    }

    #[test]
    fn test_deserialise_coinbase_message() {
        struct TestCase {
            input: &'static str,
            expected: Result<CoinbaseMessage, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: invalid CoinbaseMessage w/ unknown tag
                input: r#"{"type": "unknown", "sequence": 50,"product_id": "BTC-USD"}"#,
                expected: Err(SocketError::Serde {
                    error: serde_json::Error::custom(""),
                    payload: "".to_owned(),
                }),
            },
            TestCase {
                // TC1: valid CoinbaseMessage Spot trades
                input: r#"{
                    "type": "match","trade_id": 10,"sequence": 50,
                    "maker_order_id": "ac928c66-ca53-498f-9c13-a110027a60e8",
                    "taker_order_id": "132fb6ae-456b-4654-b4e0-d681ac05cea1",
                    "time": "2014-11-07T08:19:27.028459Z",
                    "product_id": "BTC-USD", "size": "5.23512", "price": "400.23", "side": "sell"
                }"#,
                expected: Ok(CoinbaseMessage::Trade(CoinbaseTrade {
                    subscription_id: SubscriptionId::from("matches|BTC-USD"),
                    id: 10,
                    price: 400.23,
                    quantity: 5.23512,
                    side: Side::Sell,
                    time: DateTime::from_utc(
                        NaiveDateTime::from_str("2014-11-07T08:19:27.028459").unwrap(),
                        Utc,
                    ),
                })),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<CoinbaseMessage>(test.input);
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
