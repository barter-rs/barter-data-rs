use crate::{
    exchange::{datetime_utc_from_epoch_duration, extract_next},
    model::{DataKind, PublicTrade},
    ExchangeId, MarketEvent, Validator,
};
use barter_integration::{
    error::SocketError,
    model::{Exchange, Instrument, Side, SubscriptionId},
};
use chrono::{DateTime, Utc};
use serde::{de, Deserialize, Deserializer, Serialize};
use std::time::Duration;

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
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum KrakenSubResponse {
    Subscribed {
        #[serde(rename = "channelID")]
        channel_id: u64,
        #[serde(rename = "channelName")]
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
    #[serde(rename = "errorMessage")]
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

/// `Kraken` message variants that can be received over [`WebSocket`].
///
/// See docs: <https://docs.kraken.com/websockets/#overview>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum KrakenMessage {
    Trades(KrakenTrades),
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

/// `Kraken` trade message.
///
/// See docs: <https://docs.kraken.com/websockets/#message-trade>
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize)]
pub struct KrakenTrade {
    pub price: f64,
    pub quantity: f64,
    pub time: DateTime<Utc>,
    pub side: Side,
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

        // Use Visitor implementation to deserialise the KrakenTrades WebSocket message
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::Error;

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
                // TC2: valid KrakenMessage Heartbeat
                input: r#"{"event": "heartbeat"}"#,
                expected: Ok(KrakenMessage::KrakenEvent(KrakenEvent::Heartbeat)),
            },
            TestCase {
                // TC3: valid KrakenMessage Error
                input: r#"{"errorMessage": "Malformed request", "event": "error"}"#,
                expected: Ok(KrakenMessage::KrakenEvent(KrakenEvent::Error(
                    KrakenError {
                        message: "Malformed request".to_string(),
                    },
                ))),
            },
            TestCase {
                // TC4: invalid KrakenMessage gibberish
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
}
