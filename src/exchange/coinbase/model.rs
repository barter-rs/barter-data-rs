use super::Coinbase;
use super::error::CoinbaseMsgError;
use crate::{
    exchange::de_str,
    model::{DataKind, Level, LevelDelta, OrderBook, OrderBookDelta, PublicTrade,
            OrderBookEvent, Order},
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
    #[serde(alias = "snapshot")]
    OrderBookL2Snapshot(CoinbaseOrderBookL2Snapshot),
    #[serde(alias = "l2update")]
    OrderBookL2Update(CoinbaseOrderBookL2Update),
    #[serde(alias = "l3update")]
    OrderbookL3Update(CoinbaseOrderBookL3Update),
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

/// Todo:
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#level2-channel>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CoinbaseOrderBookL2Snapshot {
    #[serde(alias = "product_id", deserialize_with = "de_ob_l2_subscription_id")]
    pub subscription_id: SubscriptionId,
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
}

impl From<(ExchangeId, Instrument, CoinbaseOrderBookL2Snapshot)> for MarketEvent {
    fn from(
        (exchange_id, instrument, ob_snapshot): (
            ExchangeId,
            Instrument,
            CoinbaseOrderBookL2Snapshot,
        ),
    ) -> Self {
        Self {
            exchange_time: Utc::now(),
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::OrderBook(OrderBook {
                last_update_id: 0,  // todo: fix
                bids: ob_snapshot.bids,
                asks: ob_snapshot.asks,
            }),
        }
    }
}

/// Todo:
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#level2-channel>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CoinbaseOrderBookL2Update {
    #[serde(alias = "product_id", deserialize_with = "de_ob_l2_subscription_id")]
    pub subscription_id: SubscriptionId,
    pub time: DateTime<Utc>,
    #[serde(alias = "changes")]
    pub deltas: Vec<CoinbaseL2LevelDelta>,
}

/// Todo:
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#level2-channel>
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CoinbaseL2LevelDelta {
    pub side: Side,
    #[serde(deserialize_with = "crate::exchange::de_str")]
    pub price: f64,
    #[serde(deserialize_with = "crate::exchange::de_str")]
    pub quantity: f64,
}

impl From<(ExchangeId, Instrument, CoinbaseOrderBookL2Update)> for MarketEvent {
    fn from(
        (exchange_id, instrument, update): (ExchangeId, Instrument, CoinbaseOrderBookL2Update),
    ) -> Self {
        // Todo: Test this functionality & make it more efficient. Also, should we be sorting the deltas?
        let (bid_deltas, ask_deltas) = update.deltas.into_iter().fold(
            (vec![], vec![]),
            |(mut bid_deltas, mut ask_deltas), delta| {
                let CoinbaseL2LevelDelta {
                    side,
                    price,
                    quantity,
                } = delta;
                let delta = LevelDelta::new(price, quantity);

                match side {
                    Side::Buy => bid_deltas.push(delta),
                    Side::Sell => ask_deltas.push(delta),
                };

                (bid_deltas, ask_deltas)
            },
        );

        Self {
            exchange_time: update.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::OrderBookDelta(OrderBookDelta {
                update_id: 0, // todo
                bid_deltas,
                ask_deltas,
            }),
        }
    }
}

/// Todo:
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#full-channel>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CoinbaseOrderBookL3Update {
    #[serde(alias = "product_id", deserialize_with = "de_ob_l3_subscription_id")]
    pub subscription_id: SubscriptionId,
    pub event_type: Option<String>,
    pub reason: Option<String>,
    pub sequence: Option<u64>,
    pub order_id: Option<String>,
    pub side: Option<Side>,
    pub price: Option<f64>,
    pub size: Option<f64>,              // only in received order messages
    pub remaining_size: Option<f64>,    // only in open order messages
    pub old_size: Option<f64>,          // only in update order messages - may be useless
    pub new_size: Option<f64>,          // only in update order messages
    pub time: DateTime<Utc>,
}

impl From<(ExchangeId, Instrument, CoinbaseOrderBookL3Update)> for MarketEvent {
    fn from(
        (exchange_id, instrument, update): (ExchangeId, Instrument, CoinbaseOrderBookL3Update),
    ) -> Self {

        Self {
            exchange_time: update.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::OrderBookEvent(
                OrderBookEvent::try_from(update).unwrap_or_else(|_| OrderBookEvent::Invalid)
            )
        }
    }
}

impl TryFrom<CoinbaseOrderBookL3Update> for OrderBookEvent {
    type Error = CoinbaseMsgError;

    fn try_from(update: CoinbaseOrderBookL3Update) -> Result<Self, Self::Error> {

        if update.sequence.is_none() { return Err(CoinbaseMsgError::NoSequence) }
        if update.event_type.is_none() { return Err(CoinbaseMsgError::NoType) }
        if update.order_id.is_none() { return Err(CoinbaseMsgError::NoOrderID) }

        match update.event_type.as_ref().unwrap().as_str() {
            "received" => Ok(Self::Received),  // todo

            "open" => {
                match (update.side, update.price, update.remaining_size) {
                    (Some(side), Some(price), Some(size)) => {
                        Ok(Self::Open(Order { id: update.order_id.unwrap(), side, price, size }))
                    },
                    _ => Err(CoinbaseMsgError::InvalidOpen)
                }
            },

            "done" => { Ok(Self::Close(update.order_id.unwrap().clone())) }

            "change" => {
                match update.new_size {
                    Some(size) => Ok(Self::Update((update.order_id.unwrap(), size))),
                    None => Err(CoinbaseMsgError::InvalidUpdate)
                }
            }

            _ => Err(CoinbaseMsgError::UnhandledType(update.event_type.unwrap().clone()))
        }
    }
}



/// Todo:
pub fn de_trade_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    serde::de::Deserialize::deserialize(deserializer)
        .map(|product_id| Coinbase::subscription_id(Coinbase::CHANNEL_TRADES, product_id))
}

/// Todo:
pub fn de_ob_l2_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    serde::de::Deserialize::deserialize(deserializer)
        .map(|product_id| Coinbase::subscription_id(Coinbase::CHANNEL_ORDER_BOOK_L2, product_id))
}

/// Todo:
pub fn de_ob_l3_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    serde::de::Deserialize::deserialize(deserializer)
        .map(|product_id| Coinbase::subscription_id(Coinbase::CHANNEL_ORDER_BOOK_L3, product_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;
    use serde::de::Error;
    use std::str::FromStr;
    use barter_integration::model::Side::Sell;

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
    fn test_deserialize_coinbase_L3_updates() {
        struct TestCase {
            input: &'static str,
            expected: Result<CoinbaseMessage, SocketError>
        }

        let cases = vec![
            TestCase {
                // TCO: valid CoinbaseMessage Order Close
                input: r#"{"order_id": "2a878d12-d790-4ea2-99ab-14c7481ab63c", "reason": "canceled",
                 "price": "1318.65", "remaining_size": "3.18557088", "type": "done", "side": "sell",
                 "product_id": "ETH-USD", "time": "2022-09-27T19:31:30.580366Z", "sequence": 36673387904}"#,
                expected: Ok(CoinbaseMessage::OrderbookL3Update(
                    CoinbaseOrderBookL3Update {
                        subscription_id: SubscriptionId::from("full|ETH-USD"),
                        sequence: Some(36673387904),
                        order_id: "2a878d12-d790-4ea2-99ab-14c7481ab63c".to_owned(),
                        side: Some(Side::Sell),
                        price: Some(1318.65),
                        size: Some(3.18557088),
                        time: DateTime::from_utc(
                            NaiveDateTime::from_str("2022-09-27T19:31:30.580366Z").unwrap(),
                            Utc,
                        ),
                    }
                ))
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
            TestCase {
                // TC2: valid CoinbaseMessage Spot OrderBookL2Snapshot
                input: r#"{"type": "snapshot","product_id": "BTC-USD","bids": [["10101.10", "0.45054140"]],"asks": [["10102.55", "0.57753524"]]}"#,
                expected: Ok(CoinbaseMessage::OrderBookL2Snapshot(
                    CoinbaseOrderBookL2Snapshot {
                        subscription_id: SubscriptionId::from("level2|BTC-USD"),
                        bids: vec![Level::new(10101.10, 0.45054140)],
                        asks: vec![Level::new(10102.55, 0.57753524)],
                    },
                )),
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
                expected: Ok(CoinbaseMessage::OrderBookL2Update(
                    CoinbaseOrderBookL2Update {
                        subscription_id: SubscriptionId::from("level2|BTC-USD"),
                        time: DateTime::from_str("2022-09-04T12:41:41.258672Z").unwrap(),
                        deltas: vec![CoinbaseL2LevelDelta {
                            side: Side::Buy,
                            price: 10101.80000000,
                            quantity: 0.162567,
                        }],
                    },
                )),
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
                expected: Ok(CoinbaseMessage::OrderBookL2Update(
                    CoinbaseOrderBookL2Update {
                        subscription_id: SubscriptionId::from("level2|BTC-USD"),
                        time: DateTime::from_str("2022-09-04T12:41:41.258672Z").unwrap(),
                        deltas: vec![
                            CoinbaseL2LevelDelta {
                                side: Side::Buy,
                                price: 22356.270000,
                                quantity: 0.0,
                            },
                            CoinbaseL2LevelDelta {
                                side: Side::Sell,
                                price: 23356.300000,
                                quantity: 1.0,
                            },
                        ],
                    },
                )),
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
