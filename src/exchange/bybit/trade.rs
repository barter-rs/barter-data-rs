use crate::{
    event::{MarketEvent, MarketIter},
    exchange::{bybit::message::BybitMessage, ExchangeId},
    subscription::trade::PublicTrade,
};
use barter_integration::model::{Exchange, Instrument, Side};
use chrono::{DateTime, Utc};
use serde::{
    de::{Error, Unexpected},
    Deserialize, Serialize,
};

/// ### Raw Payload Examples
/// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/trade>
/// Spot Side::Buy Trade
///```json
/// {
///     "T": 1672304486865,
///     "s": "BTCUSDT",
///     "S": "Buy",
///     "v": "0.001",
///     "p": "16578.50",
///     "L": "PlusTick",
///     "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
///     "BT": false
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitTrade {
    #[serde(
        alias = "T",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,

    #[serde(rename = "s")]
    pub market: String,

    #[serde(rename = "S", deserialize_with = "de_side")]
    pub side: Side,

    #[serde(alias = "v", deserialize_with = "barter_integration::de::de_str")]
    pub amount: f64,

    #[serde(alias = "p", deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,

    #[serde(rename = "L", skip)]
    pub direction: String,

    #[serde(rename = "i")]
    pub id: String,

    #[serde(rename = "BT", skip)]
    pub block_trade: bool,
}

/// Terse type alias for an [`BybitTrade`](BybitTrade) real-time trades WebSocket message.
pub type BybitTradePayload = BybitMessage<Vec<BybitTrade>>;

/// Deserialize a [`BybitTrade`] "side" string field to a Barter [`Side`].
///
/// Variants:
/// String("Sell") => Side::Sell
/// String("Buy") => Side::Buy
pub fn de_side<'de, D>(deserializer: D) -> Result<Side, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let input = <&str as serde::Deserialize>::deserialize(deserializer)?;
    let expected = "Buy | Sell";

    match input {
        "Buy" => Ok(Side::Buy),
        "Sell" => Ok(Side::Sell),
        _ => Err(Error::invalid_value(Unexpected::Str(input), &expected)),
    }
}

impl From<(ExchangeId, Instrument, BybitTradePayload)> for MarketIter<PublicTrade> {
    fn from(
        (exchange_id, instrument, trades): (ExchangeId, Instrument, BybitTradePayload),
    ) -> Self {
        let market_events = trades
            .data
            .into_iter()
            .map(|trade| {
                let i = instrument.clone();
                Ok(MarketEvent {
                    exchange_time: trade.time,
                    received_time: Utc::now(),
                    exchange: Exchange::from(exchange_id),
                    instrument: i,
                    kind: PublicTrade {
                        id: trade.id,
                        price: trade.price,
                        amount: trade.amount,
                        side: trade.side,
                    },
                })
            })
            .collect();

        Self(market_events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use barter_integration::{
            de::datetime_utc_from_epoch_duration, error::SocketError, model::SubscriptionId,
        };
        use std::time::Duration;

        #[test]
        fn test_bybit_trade() {
            struct TestCase {
                input: &'static str,
                expected: Result<BybitTrade, SocketError>,
            }

            let tests = vec![
                TestCase {
                    input: r#"
                        {
                            "T": 1672304486865,
                            "s": "BTCUSDT",
                            "S": "Buy",
                            "v": "0.001",
                            "p": "16578.50",
                            "L": "PlusTick",
                            "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
                            "BT": false
                        }
                    "#,
                    expected: Ok(BybitTrade {
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1672304486865,
                        )),
                        market: "BTCUSDT".to_string(),
                        side: Side::Buy,
                        amount: 0.001,
                        price: 16578.50,
                        id: "20f43950-d8dd-5b31-9112-a178eb6023af".to_string(),
                        direction: "".to_string(),
                        block_trade: false,
                    }),
                },
                TestCase {
                    input: r#"
                        {
                            "T": 1672304486865,
                            "s": "BTCUSDT",
                            "S": "Sell",
                            "v": "0.001",
                            "p": "16578.50",
                            "L": "PlusTick",
                            "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
                            "BT": false
                        }
                    "#,
                    expected: Ok(BybitTrade {
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1672304486865,
                        )),
                        market: "BTCUSDT".to_string(),
                        side: Side::Sell,
                        amount: 0.001,
                        price: 16578.50,
                        id: "20f43950-d8dd-5b31-9112-a178eb6023af".to_string(),
                        direction: "".to_string(),
                        block_trade: false,
                    }),
                },
                TestCase {
                    input: r#"
                        {
                            "T": 1672304486865,
                            "s": "BTCUSDT",
                            "S": "Unknown",
                            "v": "0.001",
                            "p": "16578.50",
                            "L": "PlusTick",
                            "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
                            "BT": false
                        }
                    "#,
                    expected: Err(SocketError::Unsupported {
                        entity: "",
                        item: "".to_string(),
                    }),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<BybitTrade>(test.input);
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
        fn test_bybit_trade_payload() {
            struct TestCase {
                input: &'static str,
                expected: Result<BybitTradePayload, SocketError>,
            }

            let tests = vec![
                TestCase {
                    input: r#"
                        {
                        "topic": "publicTrade.BTCUSDT",
                        "type": "snapshot",
                        "ts": 1672304486868,
                            "data": [
                                {
                                    "T": 1672304486865,
                                    "s": "BTCUSDT",
                                    "S": "Buy",
                                    "v": "0.001",
                                    "p": "16578.50",
                                    "L": "PlusTick",
                                    "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
                                    "BT": false
                                },
                                {
                                    "T": 1672304486865,
                                    "s": "BTCUSDT",
                                    "S": "Sell",
                                    "v": "0.001",
                                    "p": "16578.50",
                                    "L": "PlusTick",
                                    "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
                                    "BT": false
                                }
                            ]
                        }
                    "#,
                    expected: Ok(BybitTradePayload {
                        subscription_id: SubscriptionId("publicTrade|BTCUSDT".to_string()),
                        r#type: "snapshot".to_string(),
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1672304486868,
                        )),
                        data: vec![
                            BybitTrade {
                                time: datetime_utc_from_epoch_duration(Duration::from_millis(
                                    1672304486865,
                                )),
                                market: "BTCUSDT".to_string(),
                                side: Side::Buy,
                                amount: 0.001,
                                price: 16578.50,
                                id: "20f43950-d8dd-5b31-9112-a178eb6023af".to_string(),
                                direction: "".to_string(),
                                block_trade: false,
                            },
                            BybitTrade {
                                time: datetime_utc_from_epoch_duration(Duration::from_millis(
                                    1672304486865,
                                )),
                                market: "BTCUSDT".to_string(),
                                side: Side::Sell,
                                amount: 0.001,
                                price: 16578.50,
                                id: "20f43950-d8dd-5b31-9112-a178eb6023af".to_string(),
                                direction: "".to_string(),
                                block_trade: false,
                            },
                        ],
                        cs: None,
                    }),
                },
                TestCase {
                    input: r#"
                        {
                            "data": [
                                {
                                    "T": 1672304486865,
                                    "s": "BTCUSDT",
                                    "S": "Unknown",
                                    "v": "0.001",
                                    "p": "16578.50",
                                    "L": "PlusTick",
                                    "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
                                    "BT": false
                                }
                            ]
                        }
                    "#,
                    expected: Err(SocketError::Unsupported {
                        entity: "",
                        item: "".to_string(),
                    }),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<BybitTradePayload>(test.input);
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
}
