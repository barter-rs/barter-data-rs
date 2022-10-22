use crate::exchange::ftx::Ftx;
use crate::{
    model::{DataKind, PublicTrade},
    ExchangeId, MarketEvent,
};
use barter_integration::{
    error::SocketError,
    model::{Exchange, Instrument, Side, SubscriptionId},
    Validator,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// [`Ftx`](super::Ftx) message received in response to WebSocket subscription requests.
///
/// eg/ FtxResponse::Subscribed {"type": "subscribed", "channel": "trades", "market": "BTC/USDT"}
/// eg/ FtxResponse::Error {"type": "error", "code": 400, "msg": "Missing parameter \"channel\""}
///
/// See docs: <https://docs.ftx.com/#response-format>
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FtxSubResponse {
    Subscribed { channel: String, market: String },
    Error { msg: String },
}

impl Validator for FtxSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self {
            FtxSubResponse::Subscribed { .. } => Ok(self),
            FtxSubResponse::Error { msg } => Err(SocketError::Subscribe(format!(
                "received failure subscription response: {}",
                msg
            ))),
        }
    }
}

/// [`Ftx`](super::Ftx) message variants that can be received over [`WebSocket`](crate::WebSocket).
///
/// See docs: <https://docs.ftx.com/#public-channels>
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(tag = "channel", rename_all = "lowercase")]
pub enum FtxMessage {
    Trades {
        #[serde(rename = "market", deserialize_with = "de_trade_subscription_id")]
        subscription_id: SubscriptionId,
        #[serde(alias = "data")]
        trades: Vec<FtxTrade>,
    },
}

impl From<&FtxMessage> for SubscriptionId {
    fn from(message: &FtxMessage) -> Self {
        match message {
            FtxMessage::Trades {
                subscription_id, ..
            } => subscription_id.clone(),
        }
    }
}

/// [`Ftx`](super::Ftx) trade message.
///
/// See docs: <https://docs.ftx.com/#trades>
#[derive(Clone, Copy, PartialEq, Debug, Deserialize)]
pub struct FtxTrade {
    pub id: u64,
    pub price: f64,
    pub size: f64,
    pub side: Side,
    pub time: DateTime<Utc>,
}

impl From<(ExchangeId, Instrument, FtxTrade)> for MarketEvent {
    fn from((exchange, instrument, trade): (ExchangeId, Instrument, FtxTrade)) -> Self {
        Self {
            exchange_time: trade.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange.as_str()),
            instrument,
            kind: DataKind::Trade(PublicTrade {
                id: trade.id.to_string(),
                price: trade.price,
                quantity: trade.size,
                side: trade.side,
                sequence: None,
            }),
        }
    }
}

/// Deserialize a [`FtxMessage::Trades`](FtxMessage) "market" (eg/ "BTC/USD") as the
/// associated [`SubscriptionId`] (eg/ "trades|BTC/USD").
pub fn de_trade_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    serde::de::Deserialize::deserialize(deserializer)
        .map(|market| Ftx::subscription_id(Ftx::CHANNEL_TRADES, market))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;
    use serde::de::Error;
    use std::str::FromStr;

    #[test]
    fn test_deserialise_ftx_subscription_response() {
        struct TestCase {
            input: &'static str,
            expected: Result<FtxSubResponse, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is Subscribed
                input: r#"{"type": "subscribed", "channel": "trades", "market": "BTC/USDT"}"#,
                expected: Ok(FtxSubResponse::Subscribed {
                    channel: "trades".to_owned(),
                    market: "BTC/USDT".to_owned(),
                }),
            },
            TestCase {
                // TC1: input response is Error
                input: r#"{"type": "error", "code": 400, "msg": "Missing parameter \"channel\""}"#,
                expected: Ok(FtxSubResponse::Error {
                    msg: "Missing parameter \"channel\"".to_owned(),
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
            let actual = serde_json::from_str::<FtxSubResponse>(test.input);
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
    fn test_validate_ftx_subscription_response() {
        struct TestCase {
            input_response: FtxSubResponse,
            is_valid: bool,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is Subscribed
                input_response: FtxSubResponse::Subscribed {
                    channel: "".to_owned(),
                    market: "".to_owned(),
                },
                is_valid: true,
            },
            TestCase {
                // TC1: input response is Error
                input_response: FtxSubResponse::Error {
                    msg: "error message".to_owned(),
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
    fn test_deserialise_ftx_message() {
        struct TestCase {
            input: &'static str,
            expected: Result<FtxMessage, SocketError>,
        }

        // {"channel":"trades","market":"BTC-PERP","type":"update","data":[
        // {"id":4905794647,"price":19209.0,"size":0.03,"side":"buy","liquidation":false,"time":"2022-09-08T19:09:15.452462+00:00"},
        // {"id":4905794648,"price":19209.0,"size":0.5178,"side":"buy","liquidation":false,"time":"2022-09-08T19:09:15.452462+00:00"},
        // {"id":4905794649,"price":19209.0,"size":0.0025,"side":"buy","liquidation":false,"time":"2022-09-08T19:09:15.452462+00:00"}]}

        let cases = vec![
            TestCase {
                // TC0: input trades message is valid
                input: r#"{"channel": "trades", "market": "BTC/USDT", "type": "update", "data":
                [{"id": 3689226514, "price": 10000.0, "size": 1.0, "side": "buy", "liquidation": false,
                "time": "2022-04-06T15:38:16.182802+00:00"}]}"#,
                expected: Ok(FtxMessage::Trades {
                    subscription_id: SubscriptionId::from("trades|BTC/USDT"),
                    trades: vec![FtxTrade {
                        id: 3689226514,
                        price: 10000.0,
                        size: 1.0,
                        side: Side::Buy,
                        time: DateTime::from_utc(
                            NaiveDateTime::from_str("2022-04-06T15:38:16.182802").unwrap(),
                            Utc,
                        ),
                    }],
                }),
            },
            TestCase {
                // TC1: input trades message has invalid tag
                input: r#"{"channel": "unknown", "market": "BTC/USDT", "type": "update", "data": []}"#,
                expected: Err(SocketError::Serde {
                    error: serde_json::Error::custom(""),
                    payload: "".to_owned(),
                }),
            },
            TestCase {
                // TC2: input trades message data is malformed gibberish
                input: r#"{"channel": "trades", "market": "BTC/USDT", "type": "update", "data": [gibberish]}"#,
                expected: Err(SocketError::Serde {
                    error: serde_json::Error::custom(""),
                    payload: "".to_owned(),
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<FtxMessage>(test.input);
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
