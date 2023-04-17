use crate::{
    event::{MarketEvent, MarketIter},
    exchange::{bybit::message::BybitPayload, ExchangeId},
    subscription::liquidation::Liquidation,
};
use barter_integration::model::{Exchange, Instrument, Side};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Terse type alias for an [`BybitLiquidation`](BybitLiquidationInner) real-time liquidations WebSocket message.
pub type BybitLiquidation = BybitPayload<BybitLiquidationInner>;

/// ### Raw Payload Examples
/// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/liquidation>
///```json
/// {
///     "price": "0.03803",
///     "side": "Buy",
///     "size": "1637",
///     "symbol": "GALAUSDT",
///     "updatedTime": 1673251091822
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitLiquidationInner {
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,
    pub side: Side,
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub size: f64,
    pub symbol: String,
    #[serde(
        rename = "updatedTime",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
}

impl From<(ExchangeId, Instrument, BybitLiquidation)> for MarketIter<Liquidation> {
    fn from(
        (exchange_id, instrument, liquidation): (ExchangeId, Instrument, BybitLiquidation),
    ) -> Self {
        Self(vec![Ok(MarketEvent {
            exchange_time: liquidation.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: Liquidation {
                side: liquidation.data.side,
                price: liquidation.data.price,
                quantity: liquidation.data.size,
                time: liquidation.data.time,
            },
        })])
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
        fn test_bybit_liquidation() {
            struct TestCase {
                input: &'static str,
                expected: Result<BybitLiquidationInner, SocketError>,
            }

            let tests = vec![
                // TC0: input BybitLiquidationInner is deserialised
                TestCase {
                    input: r#"
                        {
                            "price": "0.03803",
                            "side": "Buy",
                            "size": "1637.1",
                            "symbol": "GALAUSDT",
                            "updatedTime": 1673251091822
                        }
                    "#,
                    expected: Ok(BybitLiquidationInner {
                        price: 0.03803,
                        side: Side::Buy,
                        size: 1637.1,
                        symbol: "GALAUSDT".to_string(),
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1673251091822,
                        )),
                    }),
                },
                // TC1: input BybitLiquidationInner is unable to be deserialised
                TestCase {
                    input: r#"
                          {
                            "price": 0.03803,
                            "side": "Buy",
                            "size": "1637",
                            "symbol": "GALAUSDT",
                            "updatedTime": 1673251091822
                        }
                    "#,
                    expected: Err(SocketError::Unsupported {
                        entity: "",
                        item: "".to_string(),
                    }),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<BybitLiquidationInner>(test.input);
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
        fn test_bybit_liquidation_payload() {
            struct TestCase {
                input: &'static str,
                expected: Result<BybitLiquidation, SocketError>,
            }

            let tests = vec![
                // TC0: input BybitLiquidation is deserialised
                TestCase {
                    input: r#"
                        {
                            "topic": "liquidation.GALAUSDT",
                            "type": "snapshot",
                            "ts": 1673251091822,
                            "data": {
                                "price": "0.03803",
                                "side": "Buy",
                                "size": "1637.1",
                                "symbol": "GALAUSDT",
                                "updatedTime": 1673251091822
                            }
                        }
                    "#,
                    expected: Ok(BybitLiquidation {
                        subscription_id: SubscriptionId("liquidation|GALAUSDT".to_string()),
                        r#type: "snapshot".to_string(),
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1673251091822,
                        )),
                        data: BybitLiquidationInner {
                            price: 0.03803,
                            side: Side::Buy,
                            size: 1637.1,
                            symbol: "GALAUSDT".to_string(),
                            time: datetime_utc_from_epoch_duration(Duration::from_millis(
                                1673251091822,
                            )),
                        },
                    }),
                },
                // TC1: input BybitLiquidation is invalid w/ no subscription_id
                TestCase {
                    input: r#"
                        {
                            "type": "snapshot",
                            "ts": 1673251091822,
                            "data": {
                                "price": "0.03803",
                                "side": "Buy",
                                "size": "1637",
                                "symbol": "GALAUSDT",
                                "updatedTime": 1673251091822
                            }
                        }
                    "#,
                    expected: Err(SocketError::Unsupported {
                        entity: "",
                        item: "".to_string(),
                    }),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<BybitLiquidation>(test.input);
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
