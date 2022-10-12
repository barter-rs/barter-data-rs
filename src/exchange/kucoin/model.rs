use barter_integration::{Validator, error::SocketError, model::{SubscriptionId, Side, Instrument, Exchange}};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{exchange::datetime_utc_from_epoch_duration, ExchangeId, model::{MarketEvent, PublicTrade}};

use super::Kucoin;


/// [`Kucoin`](super::Kucoin) message variants in response to Websocket subscription request.
/// 
/// ## Examples
/// Successful subscription response
/// ```
/// {
///     "id":"1665461539573",
///     "type":"ack"
/// }
/// ```
/// 
/// Error subscription response
/// ```
/// {
///     "id":"1665514264414",
///     "type":"error",
///     "code":404,
///     "data":"topic /market/match:ETH-UrSDT is not found"
/// }
/// ```
/// 
/// See docs: <https://docs.kucoin.com/#subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize, Copy)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum KucoinSubResponse {
    /// Success response to a connection request.
    #[serde( rename = "ack")]
    Subscribed,
    /// Error response to a connection request.
    Error,
}

impl Validator for KucoinSubResponse {
    fn validate(self) -> Result<Self, barter_integration::error::SocketError>
    where
        Self: Sized,
    {
        match &self {
            KucoinSubResponse::Subscribed => Ok(self),
            KucoinSubResponse::Error => Err(SocketError::Subscribe(format!(
                "Received failure subscription response from Kucoin")))
        }
    }
}

/// [`Kucoin`](super::Kucoin) message received over [`WebSocket`](crate::WebSocket) relating
/// to an active [`Subscription`](crate::Subscription). The message is associated with the original
/// [`Subscription`](crate::Subscription) using the `subscription_id` field of the 
/// respective message type as the [`SubscriptionId`](barter_integration::model::SubscriptionId).
///
/// See docs: <https://docs.kucoin.com/#websocket-feed>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
#[serde(rename_all = "camelCase", tag = "subject")]
pub enum KucoinMessage {
    /// Real-time trade messages.
    #[serde(rename = "trade.l3match")]
    Trade {
        /// The [`SubscriptionId`](barter_integration::model::SubscriptionId) associated with this message,
        #[serde(rename = "topic", deserialize_with = "de_kucoin_trade_subscription_id")]
        subscription_id: SubscriptionId,
        /// The [`KucoinTrade`] contained within this message.
        #[serde(alias = "data")]
        trade: KucoinTrade,
    }
}

impl From<&KucoinMessage> for SubscriptionId {
    fn from(message: &KucoinMessage) -> Self {
        match message {
            KucoinMessage::Trade { subscription_id, ..
            } => subscription_id.clone(),
        }
    }
}

/// [`Kucoin`](super::Kucoin) trade match. This is not aggregated.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct KucoinTrade {
    /// The sequence number of this trade.
    #[serde(deserialize_with = "crate::exchange::de_str")]
    pub sequence: u64,
    /// The client-defined order id of the passive trader.
    pub maker_order_id: String,
    /// The client-defined order id of the aggressor.
    pub taker_order_id: String,
    /// The symbol of the security traded.
    pub symbol: String,
    /// The type of trade (this will default to "match").
    #[serde(rename = "type")]
    pub trade_type: String,
    // TODO: Make sure this is actually a u64
    /// The id of this trade. This is the same as sequence number
    #[serde(deserialize_with = "crate::exchange::de_str")]
    pub trade_id: u64,
    /// The execution price of this trade.
    #[serde(deserialize_with = "crate::exchange::de_str")]
    pub price: f64,
    /// The direction of this trade as initiated by the aggressor.
    #[serde(deserialize_with = "de_side")]
    pub side: Side,
    /// The quantity transacted in this trade.
    #[serde(deserialize_with = "crate::exchange::de_str")]
    pub size: f64,
    /// The exchange-noted transaction time of this trade.
    #[serde(deserialize_with = "de_time_as_datetime_utc")]
    pub time: DateTime<Utc>,

}

impl From<(ExchangeId, Instrument, KucoinTrade)> for MarketEvent {
    fn from((exchange, instrument, trade): (ExchangeId, Instrument, KucoinTrade)) -> Self {
        Self {
            exchange_time: trade.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange.as_str()),
            instrument,
            kind: crate::model::DataKind::Trade(PublicTrade {
                id: trade.trade_id.to_string(),
                price: trade.price,
                quantity: trade.size,
                side: trade.side,
            })
        }
    }
}

fn de_kucoin_trade_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>
{
    serde::de::Deserialize::deserialize(deserializer)
        .map(|topic| Kucoin::subscription_id(topic)) 
}

/// Deserialize kucoin trade timestamp to a datetime utc.
fn de_time_as_datetime_utc<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let string_time: String = serde::de::Deserialize::deserialize(deserializer)?;
    let epoch_ms: u64 = string_time.parse().map_err(serde::de::Error::custom)?;

    Ok(datetime_utc_from_epoch_duration(std::time::Duration::from_millis(
        epoch_ms / 1_000_000,
    )))
}

/// Deserialize trade side.
fn de_side<'de, D>(deserializer: D) -> Result<Side, D::Error>
where
    D: serde::de::Deserializer<'de>
{
    serde::de::Deserialize::deserialize(deserializer)
        .map(|side: String| {
            if side == "buy" {
                Side::Buy
            } else {
                Side::Sell
            }
        })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::exchange::datetime_utc_from_epoch_duration;

    use super::*;

    #[test]
    fn test_deserialize_kucoin_subscription_response(){
        struct TestCase {
            input: &'static str,
            expected: Result<KucoinSubResponse, SocketError>,
        }
        
        let cases = vec![
            // TC0: Successful subscription
            TestCase {
                input: r#"{"id":"1665461539573","type":"ack"}"#,
                expected: Ok(KucoinSubResponse::Subscribed),
            },
            // TC1: error subscription
            TestCase {
                input: r#"{"id":"1665514264414","type":"error","code":404,"data":"topic /market/match:ETH-UrSDT is not found"}"#,
                expected: Ok(KucoinSubResponse::Error),
            }
        ];
        
        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<KucoinSubResponse>(test.input);
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
    fn test_deserialize_kucoin_message() {
        struct TestCase {
            input: &'static str,
            expected: Result<KucoinMessage, SocketError>,
        }

        let cases = vec![
            // TC0: trade message buy
            TestCase {
                input: r#"{
                    "type":"message",
                    "topic":"/market/match:ETH-USDT",
                    "subject":"trade.l3match",
                    "data":{
                        "makerOrderId":"6345c06e0cf8f8000130dc9e",
                        "price":"1277.67",
                        "sequence":"248347051968522",
                        "side":"buy",
                        "size":"0.08",
                        "symbol":"ETH-USDT",
                        "takerOrderId":"6345c071fe7a7e000154496f",
                        "time":"1665515633638000000",
                        "tradeId":"248347051968522",
                        "type":"match"
                        }
                    }"#,
                expected: Ok(KucoinMessage::Trade {
                    subscription_id: SubscriptionId::from("/market/match:ETH-USDT"),
                    trade: KucoinTrade {
                        sequence: 248347051968522, 
                        maker_order_id: String::from("6345c06e0cf8f8000130dc9e"), 
                        taker_order_id: String::from("6345c071fe7a7e000154496f"), 
                        symbol: String::from("ETH-USDT"), 
                        trade_type: String::from("match"), 
                        trade_id: 248347051968522, 
                        price: 1277.67, 
                        side: Side::Buy, 
                        size: 0.08, 
                        time:  datetime_utc_from_epoch_duration(Duration::from_millis(1665515633638))
                    }
                })
            },
            // TC1: Trade message sell
            TestCase {
                input: r#"{
                    "type":"message",
                    "topic":"/market/match:ETH-USDT",
                    "subject":"trade.l3match",
                    "data":{
                        "makerOrderId":"6345c06e0cf8f8000130dc9e",
                        "price":"1277.67",
                        "sequence":"248347051968522",
                        "side":"sell",
                        "size":"0.08",
                        "symbol":"ETH-USDT",
                        "takerOrderId":"6345c071fe7a7e000154496f",
                        "time":"1665515633638000000",
                        "tradeId":"248347051968522",
                        "type":"match"
                        }
                    }"#,
                expected: Ok(KucoinMessage::Trade {
                    subscription_id: SubscriptionId::from("/market/match:ETH-USDT"),
                    trade: KucoinTrade {
                        sequence: 248347051968522, 
                        maker_order_id: String::from("6345c06e0cf8f8000130dc9e"), 
                        taker_order_id: String::from("6345c071fe7a7e000154496f"), 
                        symbol: String::from("ETH-USDT"), 
                        trade_type: String::from("match"), 
                        trade_id: 248347051968522, 
                        price: 1277.67, 
                        side: Side::Sell, 
                        size: 0.08, 
                        time:  datetime_utc_from_epoch_duration(Duration::from_millis(1665515633638))
                    }
                })
            }
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<KucoinMessage>(test.input);
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