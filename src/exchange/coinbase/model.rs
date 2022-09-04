use super::Coinbase;
use crate::{
    exchange::de_str,
    model::{DataKind, PublicTrade, Level},
    ExchangeId, MarketEvent, Validator,
};
use barter_integration::{
    error::SocketError,
    model::{Exchange, Instrument, Side, SubscriptionId},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::model::{LevelDelta, OrderBook, OrderBookDelta};

/// [`Coinbase`] message received in response to WebSocket subscription requests.
///
/// eg/ CoinbaseResponse::Subscribed {"type": "subscriptions", "channels": [{"name": "matches", "product_ids": ["BTC-USD"]}]}
/// eg/ CoinbaseResponse::Error {"type": "error", "message": "error message", "reason": "reason"}
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CoinbaseSubResponse {
    #[serde(rename = "subscriptions")]
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
    #[serde(rename = "name")]
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


// Todo: All structs contain product_id and should therefore be topic level? -> improves find_instrument too
// Todo: Deserialize straight into SubscriptionId
/// [`Coinbase`] message variants that can be received over [`WebSocket`].
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels>
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum CoinbaseMessage {
    #[serde(alias = "match", alias = "last_match")]
    Trade(CoinbaseTrade),
    #[serde(alias = "snapshot")]
    OrderBookL2Snapshot(CoinbaseOrderBookL2Snapshot),
    #[serde(alias = "l2update")]
    OrderBookL2Update(CoinbaseOrderBookL2Update)
}

impl From<&CoinbaseMessage> for SubscriptionId {
    fn from(message: &CoinbaseMessage) -> Self {
        match message {
            CoinbaseMessage::Trade(trade) => {
                Coinbase::subscription_id(Coinbase::CHANNEL_TRADES, &trade.product_id)
            },
            CoinbaseMessage::OrderBookL2Update(update) => {
                Coinbase::subscription_id(Coinbase::CHANNEL_ORDER_BOOK_L2, &update.product_id)
            },
            CoinbaseMessage::OrderBookL2Snapshot(snapshot) => {
                Coinbase::subscription_id(Coinbase::CHANNEL_ORDER_BOOK_L2, &snapshot.product_id)
            }
        }
    }
}

/// [`Coinbase`] trade message.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#match>
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct CoinbaseTrade {
    pub product_id: String,
    #[serde(rename = "trade_id")]
    pub id: u64,
    pub sequence: u64,
    pub time: DateTime<Utc>,
    #[serde(deserialize_with = "de_str")]
    pub size: f64,
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
                quantity: trade.size,
                side: trade.side,
            }),
        }
    }
}

/// Todo:
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#level2-channel>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CoinbaseOrderBookL2Snapshot {
    pub product_id: String,
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
}

impl From<(ExchangeId, Instrument, CoinbaseOrderBookL2Snapshot)> for MarketEvent {
    fn from((exchange_id, instrument, ob_snapshot): (ExchangeId, Instrument, CoinbaseOrderBookL2Snapshot)) -> Self {
        Self {
            exchange_time: Utc::now(),
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::OrderBook(OrderBook {
                bids: ob_snapshot.bids,
                asks: ob_snapshot.asks
            })
        }
    }
}

/// Todo:
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#level2-channel>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CoinbaseOrderBookL2Update {
    product_id: String,
    time: DateTime<Utc>,
    #[serde(rename = "changes")]
    deltas: Vec<LevelDelta>,
}

impl From<(ExchangeId, Instrument, CoinbaseOrderBookL2Update)> for MarketEvent {
    fn from((exchange_id, instrument, update): (ExchangeId, Instrument, CoinbaseOrderBookL2Update)) -> Self {
        Self {
            exchange_time: update.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::OrderBookDelta(OrderBookDelta {
                deltas: update.deltas,
            })
        }
    }
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
                    product_id: String::from("BTC-USD"),
                    id: 10,
                    sequence: 50,
                    price: 400.23,
                    size: 5.23512,
                    side: Side::Sell,
                    time: DateTime::from_utc(NaiveDateTime::from_str("2014-11-07T08:19:27.028459").unwrap(), Utc)
                }))
            },
            TestCase {
                // TC2: valid CoinbaseMessage Spot OrderBookL2Snapshot
                input: r#"{"type": "snapshot","product_id": "BTC-USD","bids": [["10101.10", "0.45054140"]],"asks": [["10102.55", "0.57753524"]]}"#,
                expected: Ok(CoinbaseMessage::OrderBookL2Snapshot(CoinbaseOrderBookL2Snapshot {
                    product_id: "BTC-USD".to_string(),
                    bids: vec![Level::new(10101.10, 0.45054140)],
                    asks: vec![Level::new(10102.55, 0.57753524)]
                }))
            },
            TestCase {
                // TC3: invalid CoinbaseMessage Spot OrderBookL2Snapshot w/ non-string values
                input: r#"{"type": "snapshot","product_id": "BTC-USD","bids": [[10101.10, 0.45054140]],"asks": [[10102.55, 0.57753524]]}"#,
                expected: Err(SocketError::Serde {
                    error: serde_json::Error::custom(""),
                    payload: "".to_owned(),
                }),
            },
            TestCase {
                // TC4: valid CoinbaseMessage Spot OrderBookL2Update w/ single change
                input: r#"{
                    "type": "l2update",
                    "product_id": "BTC-USD",
                    "time": "2022-09-04T12:41:41.258672Z",
                    "changes": [
                        [
                            "buy",
                            "10101.80000000",
                            "0.162567"
                        ]
                    ]
                }"#,
                expected: Ok(CoinbaseMessage::OrderBookL2Update(CoinbaseOrderBookL2Update {
                    product_id: "BTC-USD".to_string(),
                    time: DateTime::from_str("2022-09-04T12:41:41.258672Z").unwrap(),
                    deltas: vec![LevelDelta::new(Side::Buy, 10101.80000000, 0.162567)]
                }))
            },
            TestCase {
                // TC5: valid CoinbaseMessage Spot OrderBookL2Update w/ multiple changes
                input: r#"{
                    "type": "l2update",
                    "product_id": "BTC-USD",
                    "changes": [
                        [
                            "buy",
                            "22356.270000",
                            "0.00000000"
                        ],
                        [
                            "sell",
                            "23356.300000",
                            "1.00000000"
                        ]
                    ],
                    "time": "2022-09-04T12:41:41.258672Z"
                }"#,
                expected: Ok(CoinbaseMessage::OrderBookL2Update(CoinbaseOrderBookL2Update {
                    product_id: "BTC-USD".to_string(),
                    time: DateTime::from_str("2022-09-04T12:41:41.258672Z").unwrap(),
                    deltas: vec![
                        LevelDelta::new(Side::Buy, 22356.270000, 0.0),
                        LevelDelta::new(Side::Sell, 23356.300000, 1.0),
                    ]
                }))
            }
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
