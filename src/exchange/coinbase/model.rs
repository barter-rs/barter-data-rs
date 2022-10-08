use super::Coinbase;
use crate::{
    exchange::de_str,
    model::{DataKind, Level, LevelDelta, OrderBook, OrderBookDelta, PublicTrade,
            OrderBookEvent, OrderType, Order},
    ExchangeId, MarketEvent, Validator,
};
use barter_integration::{
    error::SocketError,
    model::{Exchange, Instrument, Side, SubscriptionId},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde::de::Error;

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
    #[serde(alias = "received")]
    OrderBookL3Received(CoinbaseOrderBookL3Received),
    #[serde(alias = "open")]
    OrderBookL3Open(CoinbaseOrderBookL3Open),
    #[serde(alias = "done")]
    OrderBookL3Done(CoinbaseOrderBookL3Done),
    #[serde(alias = "change")]
    OrderBookL3Change(CoinbaseOrderBookL3Change),
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
    pub sequence: u64,
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
                sequence: trade.sequence,
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

/// Used to populate optional missing key-value pairs as None during deserialization.
fn serde_default_none<T>() -> Option<T> { None }

/// Todo:
///
/// Sample Received message:
/// {"order_id": "c1662947-1b81-4637-986c-00d08fcec719", "order_type": "limit", "size": "0.06240841",
/// "price": "1497.16", "client_oid": "472eed13-793f-4f1f-a7bd-cd63b938ffdb", "type": "received",
/// "side": "buy", "product_id": "ETH-USD", "time": "2022-08-30T18:32:01.223105Z", "sequence": 35140793295}
///
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CoinbaseOrderBookL3Received {
    #[serde(alias = "product_id", deserialize_with = "de_ob_l3_subscription_id")]
    pub subscription_id: SubscriptionId,
    pub order_id: String,
    pub order_type: OrderType,
    #[serde(deserialize_with = "de_ob_l3_floats")]
    pub size: f64,
    #[serde(deserialize_with = "de_ob_l3_floats")]
    pub price: f64,
    pub client_oid: String,
    pub side: Side,
    pub time: DateTime<Utc>,
    pub sequence: u64,
}

impl From<(ExchangeId, Instrument, CoinbaseOrderBookL3Received)> for MarketEvent {
    fn from(
        (exchange_id, instrument, received): (ExchangeId, Instrument, CoinbaseOrderBookL3Received),
    ) -> Self {

        Self {
            exchange_time: received.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::OrderBookEvent(
                OrderBookEvent::from(received)
            )
        }
    }
}

impl From<CoinbaseOrderBookL3Received> for OrderBookEvent {
    fn from(received: CoinbaseOrderBookL3Received) -> Self {
        OrderBookEvent::Received(
        Order {
                id: received.order_id,
                side: received.side,
                price: received.price,
                size: received.size,
                order_type: received.order_type,
            },
    received.sequence,
        )
    }
}

/// Todo:
///
/// Sample Open message:
/// {"price": "1592.98", "order_id": "95162445-d1d0-4f36-9dc6-c30f1fe2ff5b",
/// "remaining_size": "0.00062776", "type": "open", "side": "sell", "product_id": "ETH-USD",
/// "time": "2022-08-30T18:32:01.293939Z", "sequence": 35140793354}
///
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CoinbaseOrderBookL3Open {
    #[serde(alias = "product_id", deserialize_with = "de_ob_l3_subscription_id")]
    pub subscription_id: SubscriptionId,
    pub order_id: String,
    #[serde(deserialize_with = "de_ob_l3_floats")]
    pub remaining_size: f64,
    #[serde(deserialize_with = "de_ob_l3_floats")]
    pub price: f64,
    pub side: Side,
    pub time: DateTime<Utc>,
    pub sequence: u64,
}

impl From<(ExchangeId, Instrument, CoinbaseOrderBookL3Open)> for MarketEvent {
    fn from(
        (exchange_id, instrument, open): (ExchangeId, Instrument, CoinbaseOrderBookL3Open),
    ) -> Self {

        Self {
            exchange_time: open.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::OrderBookEvent(
                OrderBookEvent::from(open)
            )
        }
    }
}

impl From<CoinbaseOrderBookL3Open> for OrderBookEvent {
    fn from(open: CoinbaseOrderBookL3Open) -> Self {
        OrderBookEvent::Open(
        Order {
                id: open.order_id,
                side: open.side,
                price: open.price,
                size: open.remaining_size,
                order_type: OrderType::Limit,
            },
    open.sequence,
        )
    }
}

/// Todo:
///
/// Sample Done message:
/// {"order_id": "a2243810-e8ec-4b63-ac90-38c4099b38e8", "reason": "canceled", "price": "1499.35",
/// "remaining_size": "1.41907", "type": "done", "side": "sell", "product_id": "ETH-USD",
/// "time": "2022-08-30T18:32:01.292484Z", "sequence": 35140793350}
///
/// Done messages occasionally leave out price, size and side keys. Fortunately, they are not necessary.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CoinbaseOrderBookL3Done {
    #[serde(alias = "product_id", deserialize_with = "de_ob_l3_subscription_id")]
    pub subscription_id: SubscriptionId,
    pub order_id: String,
    pub reason: String,
    #[serde(deserialize_with = "de_ob_l3_optional_floats", default = "serde_default_none")]
    pub remaining_size: Option<f64>,
    #[serde(deserialize_with = "de_ob_l3_optional_floats", default = "serde_default_none")]
    pub price: Option<f64>,
    #[serde(default = "serde_default_none")]
    pub side: Option<Side>,
    pub time: DateTime<Utc>,
    pub sequence: u64,
}

impl From<(ExchangeId, Instrument, CoinbaseOrderBookL3Done)> for MarketEvent {
    fn from(
        (exchange_id, instrument, done): (ExchangeId, Instrument, CoinbaseOrderBookL3Done),
    ) -> Self {
        Self {
            exchange_time: done.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::OrderBookEvent(
                OrderBookEvent::from(done)
            )
        }
    }
}

impl From<CoinbaseOrderBookL3Done> for OrderBookEvent {
    fn from(done: CoinbaseOrderBookL3Done) -> Self {
        OrderBookEvent::Done(done.order_id,done.sequence)
    }
}

/// Todo:
///
/// Sample Change message:
/// {"price": "1679.23", "old_size": "9.01473019", "new_size": "4.0147302",
/// "order_id": "d8b8fef6-62f2-435d-b70b-a6f077da05f0", "reason": "STP", "type": "change",
/// "side": "buy", "product_id": "ETH-USD", "time": "2022-08-26T00:17:41.266856Z",
/// "sequence": 34685643156}
///
/// Change messages are infrequent. It's possible that price or side may not be included.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CoinbaseOrderBookL3Change {
    #[serde(alias = "product_id", deserialize_with = "de_ob_l3_subscription_id")]
    pub subscription_id: SubscriptionId,
    pub order_id: String,
    pub reason: String,
    #[serde(deserialize_with = "de_ob_l3_optional_floats", default = "serde_default_none")]
    pub old_size: Option<f64>,
    #[serde(deserialize_with = "de_ob_l3_floats")]
    pub new_size: f64,
    #[serde(deserialize_with = "de_ob_l3_optional_floats", default = "serde_default_none")]
    pub price: Option<f64>,
    #[serde(default = "serde_default_none")]
    pub side: Option<Side>,
    pub time: DateTime<Utc>,
    pub sequence: u64,
}

impl From<(ExchangeId, Instrument, CoinbaseOrderBookL3Change)> for MarketEvent {
    fn from(
        (exchange_id, instrument, change): (ExchangeId, Instrument, CoinbaseOrderBookL3Change),
    ) -> Self {
        Self {
            exchange_time: change.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::OrderBookEvent(
                OrderBookEvent::from(change)
            )
        }
    }
}

impl From<CoinbaseOrderBookL3Change> for OrderBookEvent {
    fn from(change: CoinbaseOrderBookL3Change) -> Self {
        OrderBookEvent::Change(change.order_id,change.new_size, change.sequence)
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

/// Todo:
pub fn de_ob_l3_floats<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let num_str: &str = Deserialize::deserialize(deserializer)?;
    num_str.parse().map_err(|_| Error::custom("Float parsing error"))
}

/// Todo:
pub fn de_ob_l3_optional_floats<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let maybe_num_str: Option<&str> = serde::de::Deserialize::deserialize(deserializer)?;
    match maybe_num_str {
        Some(num_str) => {
            match num_str.parse::<f64>() {
                Ok(float) => Ok(Some(float)),
                Err(_) => Err(Error::custom("Float parsing error")),
            }
        },
        None => Ok(None)
    }
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
    fn test_deserialize_coinbase_l3_updates() {
        struct TestCase {
            input: &'static str,
            expected: Result<CoinbaseMessage, SocketError>
        }

        let cases = vec![
            TestCase {
                // TC0: Valid CoinbaseMessage Received
                input: r#"{"order_id": "bdfe8f61-7f29-44e4-bd80-059f3ba9c408", "order_type": "limit",
                "size": "0.00998808", "price": "1501.79", "client_oid": "3c5523dc-6885-f82c-c40b-4f57be40fee4",
                "type": "received", "side": "buy", "product_id": "ETH-USD",
                "time": "2022-08-30T18:33:16.604432Z", "sequence": 35140902732}"#,
                expected: Ok(CoinbaseMessage::OrderBookL3Received(
                    CoinbaseOrderBookL3Received {
                        subscription_id: SubscriptionId::from("full|ETH-USD"),
                        order_id: "bdfe8f61-7f29-44e4-bd80-059f3ba9c408".to_string(),
                        order_type: OrderType::Limit,
                        size: 0.00998808,
                        price: 1501.79,
                        client_oid: "3c5523dc-6885-f82c-c40b-4f57be40fee4".to_string(),
                        side: Side::Buy,
                        time: DateTime::from_utc(
                            NaiveDateTime::parse_from_str(
                                "2022-08-30T18:33:16.604432Z",
                                "%Y-%m-%dT%H:%M:%S.%6fZ",
                            ).unwrap(),
                            Utc,
                        ),
                        sequence: 35140902732,
                    }
                ))
            },
            TestCase {
                // TC1: Valid CoinbaseMessage Open
                input: r#"{"price": "1501.49", "order_id": "ee79e321-06f2-4967-8052-635bb68e1fa5",
                 "remaining_size": "0.0937345", "type": "open", "side": "buy", "product_id": "ETH-USD",
                 "time": "2022-08-30T18:33:16.741365Z", "sequence": 35140902908}"#,
                expected: Ok(CoinbaseMessage::OrderBookL3Open(
                    CoinbaseOrderBookL3Open {
                        subscription_id: SubscriptionId::from("full|ETH-USD"),
                        order_id: "ee79e321-06f2-4967-8052-635bb68e1fa5".to_string(),
                        remaining_size: 0.0937345,
                        price: 1501.49,
                        side: Side::Buy,
                        time: DateTime::from_utc(
                            NaiveDateTime::parse_from_str(
                                "2022-08-30T18:33:16.741365Z",
                                "%Y-%m-%dT%H:%M:%S.%6fZ",
                            ).unwrap(),
                            Utc,
                        ),
                        sequence: 35140902908,
                    }
                ))
            },
            TestCase {
                // TC2: Valid CoinbaseMessage Done
                input: r#"{"order_id": "2a878d12-d790-4ea2-99ab-14c7481ab63c", "reason": "canceled",
                 "price": "1318.65", "remaining_size": "3.18557088", "type": "done", "side": "sell",
                 "product_id": "ETH-USD", "time": "2022-09-27T19:31:30.580366Z", "sequence": 36673387904}"#,
                expected: Ok(CoinbaseMessage::OrderBookL3Done(
                    CoinbaseOrderBookL3Done {
                        subscription_id: SubscriptionId::from("full|ETH-USD"),
                        reason: "canceled".to_string(),
                        sequence: 36673387904,
                        order_id: "2a878d12-d790-4ea2-99ab-14c7481ab63c".to_string(),
                        side: Some(Side::Sell),
                        price: Some(1318.65),
                        remaining_size: Some(3.18557088),
                        time: DateTime::from_utc(
                            NaiveDateTime::parse_from_str(
                                "2022-09-27T19:31:30.580366Z",
                                "%Y-%m-%dT%H:%M:%S.%6fZ",
                            ).unwrap(),
                            Utc,
                        ),
                    }
                ))
            },
            TestCase {
                // TC3: Valid CoinbaseMessage Change
                input: r#"{"price": "1679.23", "old_size": "9.01473019", "new_size": "4.0147302",
                    "order_id": "d8b8fef6-62f2-435d-b70b-a6f077da05f0", "reason": "STP", "type": "change",
                    "side": "buy", "product_id": "ETH-USD", "time": "2022-08-26T00:17:41.266856Z",
                    "sequence": 34685643156}"#,
                expected: Ok(CoinbaseMessage::OrderBookL3Change(
                    CoinbaseOrderBookL3Change {
                        subscription_id: SubscriptionId::from("full|ETH-USD"),
                        reason: "STP".to_string(),
                        old_size: Some(9.01473019),
                        sequence: 34685643156,
                        order_id: "d8b8fef6-62f2-435d-b70b-a6f077da05f0".to_string(),
                        side: Some(Side::Buy),
                        price: Some(1679.23),
                        time: DateTime::from_utc(
                            NaiveDateTime::parse_from_str(
                                "2022-08-26T00:17:41.266856Z",
                                "%Y-%m-%dT%H:%M:%S.%6fZ",
                            ).unwrap(),
                            Utc,
                        ),
                        new_size: 4.0147302
                    }
                ))
            },
            TestCase {
                // TC4: Valid Coinbase Message Done with missing price, size, side
                input: r#"{"order_id": "2a878d12-d790-4ea2-99ab-14c7481ab63c", "reason": "canceled", "type": "done",
                 "product_id": "ETH-USD", "time": "2022-09-27T19:31:30.580366Z", "sequence": 36673387904}"#,
                expected: Ok(CoinbaseMessage::OrderBookL3Done(
                    CoinbaseOrderBookL3Done {
                        subscription_id: SubscriptionId::from("full|ETH-USD"),
                        reason: "canceled".to_string(),
                        sequence: 36673387904,
                        order_id: "2a878d12-d790-4ea2-99ab-14c7481ab63c".to_string(),
                        side: None,
                        price: None,
                        remaining_size: None,
                        time: DateTime::from_utc(
                            NaiveDateTime::parse_from_str(
                                "2022-09-27T19:31:30.580366Z",
                                "%Y-%m-%dT%H:%M:%S.%6fZ",
                            ).unwrap(),
                            Utc,
                        ),
                    }
                ))
            },
            TestCase {
                // TC5: Invalid CoinbaseMessage Open - incorrect keys
                input: r#"{"order_id": "2a878d12-d790-4ea2-99ab-14c7481ab63c", "reason": "canceled", "type": "open",
                 "product_id": "ETH-USD", "time": "2022-09-27T19:31:30.580366Z", "sequence": 36673387904}"#,
                expected: Err(SocketError::Serde {
                    error: serde_json::Error::custom(""),
                    payload: "".to_owned(),
                }),
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
                    sequence: 50,
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
