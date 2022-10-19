use super::Coinbase;
use crate::{
    exchange::de_str,
    model::{
        orderbook::{OrderBookEvent, AtomicOrder},
        DataKind,
        PublicTrade,
        subscription::de_floats,
    },
    ExchangeId, MarketEvent, Validator,
};
use barter_integration::{
    error::SocketError,
    model::{Exchange, Instrument, Side, SubscriptionId},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde::de::Error;
use crate::model::orderbook::{LimitOrder, MarketOrder, Order};

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
    #[serde(alias = "received")]
    OrderBookL3Received(CoinbaseOrderBookL3Received),
    #[serde(alias = "open")]
    OrderBookL3Open(CoinbaseOrderBookL3Open),
    #[serde(alias = "done")]
    OrderBookL3Done(CoinbaseOrderBookL3Done),
    #[serde(alias = "change")]
    OrderBookL3Change(CoinbaseOrderBookL3Change),
}

/// [`Coinbase`] trade message - these are sent through either the dedicated 'matches' channel,
/// or amongst orderbook delta messages in the 'full' channel.
///
/// Sample: {"trade_id": 348217732, "maker_order_id": "15d4d667-855e-43fe-8ef3-4525e28630b3",
/// "taker_order_id": "2d383d4c-8b13-481b-b4de-3dda6333423d", "size": "0.5",
/// "price": "1498.31", "type": "match", "side": "sell", "product_id": "ETH-USD",
/// "time": "2022-08-30T18:32:01.306789Z", "sequence": 35140793377}
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

/// Used to populate optional missing key-value pairs as None during deserialization.
fn serde_default_none<T>() -> Option<T> { None }

/// Enum representing order types that may be encountered in
/// an L3 orderbook delta stream's received message.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    Limit,
    Market,
}

/// Coinbase sends a Received message to indicate that its matching engine has received an order
/// and accepted it for processing.
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
    #[serde(deserialize_with = "de_ob_l3_optional_floats", default="serde_default_none")]
    pub size: Option<f64>,
    #[serde(deserialize_with = "de_ob_l3_optional_floats", default="serde_default_none")]
    pub price: Option<f64>,
    #[serde(deserialize_with = "de_ob_l3_optional_floats", default="serde_default_none")]
    pub funds: Option<f64>,
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
        match received.order_type {
            OrderType::Limit => {
                match received.side {
                    Side::Buy => OrderBookEvent::Received(Order::LimitOrder(
                        LimitOrder::Bid(
                            AtomicOrder {
                                id: received.order_id,
                                price: received.price.unwrap_or(0.0),
                                size: received.size.unwrap_or(0.0),
                            }
                        )
                    ), received.sequence),
                    Side::Sell => OrderBookEvent::Received(Order::LimitOrder(
                        LimitOrder::Ask(
                            AtomicOrder {
                                id: received.order_id,
                                price: received.price.unwrap_or(0.0),
                                size: received.size.unwrap_or(0.0),
                            }
                        )
                    ), received.sequence)
                }
            },
            OrderType::Market => {
                match received.side {
                    Side::Buy => OrderBookEvent::Received(Order::MarketOrder(
                        match (received.funds, received.size) {
                            (Some(funds), None) => MarketOrder::BuyWithFunds(funds),
                            (None, Some(size)) => MarketOrder::BuySize(size),
                            _ => MarketOrder::BuySize(0.0),
                        }
                    ), received.sequence),
                    Side::Sell => OrderBookEvent::Received(Order::MarketOrder(
                        match (received.funds, received.size) {
                            (Some(funds), None) => MarketOrder::SellForFunds(funds),
                            (None, Some(size)) => MarketOrder::SellSize(size),
                            _ => MarketOrder::SellSize(0.0),
                        }
                    ), received.sequence)
                }
            }
        }
    }
}

/// Coinbase sends an open message to indicate a new order being added to the book that
/// have not filled immediately. Remaining_size indicates how much of the order is unfilled
/// and going on the book.
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
    #[serde(deserialize_with = "de_floats")]
    pub remaining_size: f64,
    #[serde(deserialize_with = "de_floats")]
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
        let order = AtomicOrder {
            id: open.order_id,
            price: open.price,
            size: open.remaining_size,
        };
        match open.side {
            Side::Buy => OrderBookEvent::Open(
                LimitOrder::Bid(order), open.sequence),
            Side::Sell => OrderBookEvent::Open(
                LimitOrder::Ask(order), open.sequence),
        }
    }
}

/// Coinbase sends a Done message to indicate that an order has been removed from the book
/// for any reason, including cancels and fills.
///
/// Sample Done message:
/// {"order_id": "a2243810-e8ec-4b63-ac90-38c4099b38e8", "reason": "canceled", "price": "1499.35",
/// "remaining_size": "1.41907", "type": "done", "side": "sell", "product_id": "ETH-USD",
/// "time": "2022-08-30T18:32:01.292484Z", "sequence": 35140793350}
///
/// Done messages occasionally leave out price, size and side keys.
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
        OrderBookEvent::Done(done.order_id, done.sequence)
    }
}

/// Coinbase sends a Change message to indicate that an existing order's size has been changed,
/// either through Self-trade Prevention or a Modify Order Request.
///
/// Sample Change message:
/// {"price": "1679.23", "old_size": "9.01473019", "new_size": "4.0147302",
/// "order_id": "d8b8fef6-62f2-435d-b70b-a6f077da05f0", "reason": "STP", "type": "change",
/// "side": "buy", "product_id": "ETH-USD", "time": "2022-08-26T00:17:41.266856Z",
/// "sequence": 34685643156}
///
/// Change messages are infrequent and may not include price or old_size.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CoinbaseOrderBookL3Change {
    #[serde(alias = "product_id", deserialize_with = "de_ob_l3_subscription_id")]
    pub subscription_id: SubscriptionId,
    pub order_id: String,
    pub reason: String,
    #[serde(deserialize_with = "de_ob_l3_optional_floats", default = "serde_default_none")]
    pub old_size: Option<f64>,
    #[serde(deserialize_with = "de_floats")]
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
        OrderBookEvent::Change(change.order_id, change.new_size, change.sequence)
    }
}

/// Deserialize a [`CoinbaseTrade`] "product_id" (eg/ "BTC-USD") as the associated [`SubscriptionId`]
/// (eg/ SubscriptionId("matches|BTC-USD")).
pub fn de_trade_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    serde::de::Deserialize::deserialize(deserializer)
        .map(|product_id| Coinbase::subscription_id(Coinbase::CHANNEL_TRADES, product_id))
}

/// Deserialize a ['OrderBookL3Received'], ['OrderBookL3Open'], ['OrderBookL3Done'] or ['OrderBookL3Change']'s
/// "product_id" (eg/ "BTC-USD") as the associated ['SubscriptionId'] (eg/ SubscriptionId("full|BTC-USD")).
pub fn de_ob_l3_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    serde::de::Deserialize::deserialize(deserializer)
        .map(|product_id| Coinbase::subscription_id(Coinbase::CHANNEL_ORDER_BOOK_L3, product_id))
}

/// Attempt to deserialize a float that may or may not exist.
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

    #[test]
    fn test_deserialize_coinbase_l3_updates() {
        struct TestCase {
            input: &'static str,
            expected: Result<CoinbaseMessage, SocketError>
        }

        let cases = vec![
            TestCase {
                // TC0: Valid CoinbaseMessage Received Limit Buy
                input: r#"{"order_id": "bdfe8f61-7f29-44e4-bd80-059f3ba9c408", "order_type": "limit",
                "size": "0.00998808", "price": "1501.79", "client_oid": "3c5523dc-6885-f82c-c40b-4f57be40fee4",
                "type": "received", "side": "buy", "product_id": "ETH-USD",
                "time": "2022-08-30T18:33:16.604432Z", "sequence": 35140902732}"#,
                expected: Ok(CoinbaseMessage::OrderBookL3Received(
                    CoinbaseOrderBookL3Received {
                        subscription_id: SubscriptionId::from("full|ETH-USD"),
                        order_id: "bdfe8f61-7f29-44e4-bd80-059f3ba9c408".to_string(),
                        order_type: OrderType::Limit,
                        size: Some(0.00998808),
                        price: Some(1501.79),
                        funds: None,
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
                // TC1: Valid CoinbaseMessage Received Market Sell Size
                input: r#"{"order_id": "ba3148f1-877e-439e-ad77-1a3e9237b8c0", "order_type": "market",
                "size": "0.47673728", "client_oid": "2881edb9-1aee-498c-9555-d854f3f26786",
                "type": "received", "side": "sell", "product_id": "ETH-USD",
                "time": "2022-08-30T18:33:47.651822Z", "sequence": 35140939951}"#,
                expected: Ok(CoinbaseMessage::OrderBookL3Received(
                    CoinbaseOrderBookL3Received {
                        subscription_id: SubscriptionId::from("full|ETH-USD"),
                        order_id: "ba3148f1-877e-439e-ad77-1a3e9237b8c0".to_string(),
                        order_type: OrderType::Market,
                        size: Some(0.47673728),
                        price: None,
                        funds: None,
                        client_oid: "2881edb9-1aee-498c-9555-d854f3f26786".to_string(),
                        side: Side::Sell,
                        time: DateTime::from_utc(
                            NaiveDateTime::parse_from_str(
                                "2022-08-30T18:33:47.651822Z",
                                "%Y-%m-%dT%H:%M:%S.%6fZ",
                            ).unwrap(),
                            Utc,
                        ),
                        sequence: 35140939951,
                    }
                ))
            },
            TestCase {
                // TC2: Valid CoinbaseMessage Received Market Buy with Funds
                input: r#"{"order_id": "fd565fd1-806f-4f37-bb73-c5ab8d5e9ec0", "order_type": "market",
                "funds": "795.228618834", "client_oid": "57546ae1-29e2-4a23-efdf-cc1904e60b61",
                "type": "received", "side": "buy", "product_id": "ETH-USD",
                "time": "2022-08-30T18:34:06.143789Z", "sequence": 35140967372}"#,
                expected: Ok(CoinbaseMessage::OrderBookL3Received(
                    CoinbaseOrderBookL3Received {
                        subscription_id: SubscriptionId::from("full|ETH-USD"),
                        order_id: "fd565fd1-806f-4f37-bb73-c5ab8d5e9ec0".to_string(),
                        order_type: OrderType::Market,
                        size: None,
                        price: None,
                        funds: Some(795.228618834),
                        client_oid: "57546ae1-29e2-4a23-efdf-cc1904e60b61".to_string(),
                        side: Side::Buy,
                        time: DateTime::from_utc(
                            NaiveDateTime::parse_from_str(
                                "2022-08-30T18:34:06.143789Z",
                                "%Y-%m-%dT%H:%M:%S.%6fZ",
                            ).unwrap(),
                            Utc,
                        ),
                        sequence: 35140967372,
                    }
                ))
            },
            TestCase {
                // TC3: Valid CoinbaseMessage Open
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
                // TC4: Valid CoinbaseMessage Done
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
                // TC5: Valid CoinbaseMessage Change
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
                // TC6: Valid Coinbase Message Done with missing price, size, side
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
                // TC7: Invalid CoinbaseMessage Open - incorrect keys
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
}
