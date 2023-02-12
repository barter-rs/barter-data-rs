use barter_integration::model::{Exchange, Instrument, Side, Symbol};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::event::{MarketEvent, MarketIter};
use crate::exchange::bybit::message::BybitMessage;
use crate::exchange::ExchangeId;
use crate::subscription::trade::PublicTrade;

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

    #[serde(rename = "S", deserialize_with = "de_side_side")]
    pub side: Side,

    #[serde(alias = "v", deserialize_with = "barter_integration::de::de_str")]
    pub amount: f64,

    #[serde(alias = "p", deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,

    #[serde(rename = "L")]
    pub direction: String,

    #[serde(rename = "i")]
    pub id: String,

    #[serde(rename = "BT")]
    pub bt: bool,
}

/// Deserialize a [`BybitTrade`] "side" string field to a Barter [`Side`].
///
/// Variants:
/// String("Sell") => Side::Sell
/// String("Buy") => Side::Buy
pub fn de_side_side<'de, D>(deserializer: D) -> Result<Side, D::Error>
    where
        D: serde::de::Deserializer<'de>,
{
    <Symbol as Deserialize>::deserialize(deserializer).map(|side| {
        if side == "Sell".into() {
            Side::Sell
        } else {
            Side::Buy
        }
    })
}

impl From<(ExchangeId, Instrument, BybitMessage)> for MarketIter<PublicTrade> {
    fn from((exchange_id, instrument, trades): (ExchangeId, Instrument, BybitMessage)) -> Self {
        let mut market_events = vec![];
        for trade in trades.data {
            let i = instrument.clone();
            market_events.push(Ok(MarketEvent {
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
            }))
        }

        Self(market_events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use barter_integration::de::datetime_utc_from_epoch_duration;
        use barter_integration::error::SocketError;
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
                    expected: Ok(
                        BybitTrade {
                            time: datetime_utc_from_epoch_duration(Duration::from_millis(
                                1672304486865,
                            )),
                            market: String::from("BTCUSDT"),
                            side: Side::Buy,
                            amount: 0.001,
                            price: 16578.50,
                            direction: String::from("PlusTick"),
                            id: String::from("20f43950-d8dd-5b31-9112-a178eb6023af"),
                            bt: false,
                        }
                    )
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
                    expected: Ok(
                        BybitTrade {
                            time: datetime_utc_from_epoch_duration(Duration::from_millis(
                                1672304486865,
                            )),
                            market: String::from("BTCUSDT"),
                            side: Side::Sell,
                            amount: 0.001,
                            price: 16578.50,
                            direction: String::from("PlusTick"),
                            id: String::from("20f43950-d8dd-5b31-9112-a178eb6023af"),
                            bt: false,
                        }
                    )
                }
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
    }
}
