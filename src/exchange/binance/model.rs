use super::futures::BinanceFuturesUsd;
use crate::{
    exchange::{datetime_utc_from_epoch_duration},
    model::{DataKind, PublicTrade, Level, LevelDelta, OrderBookDelta},
    ExchangeId, MarketEvent,
};
use barter_integration::{
    error::SocketError,
    model::{Exchange, Instrument, Side, SubscriptionId},
    Validator,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// `Binance` & `BinanceFuturesUsd` `Subscription` response message.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#live-subscribing-unsubscribing-to-streams>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BinanceSubResponse {
    result: Option<Vec<String>>,
    id: u32,
}

impl Validator for BinanceSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        if self.result.is_none() {
            Ok(self)
        } else {
            Err(SocketError::Subscribe(
                "received failure subscription response".to_owned(),
            ))
        }
    }
}

/// `Binance` message variants that could be received over [`WebSocket`](crate::WebSocket).
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
#[serde(tag = "e", rename_all = "camelCase")]
pub enum BinanceMessage {
    #[serde(alias = "aggTrade")]
    Trade(BinanceTrade),
    #[serde(alias = "depthUpdate")]
    OrderBookL2Update(BinanceOrderBookL2Update),
}

impl From<(ExchangeId, Instrument, BinanceMessage)> for MarketEvent {
    fn from((exchange, instrument, message): (ExchangeId, Instrument, BinanceMessage)) -> Self {
        match message {
            BinanceMessage::Trade(trade) => MarketEvent::from((exchange, instrument, trade)),
            BinanceMessage::OrderBookL2Update(update) => MarketEvent::from((exchange, instrument, update))
        }
    }
}

/// `Binance` real-time trade message.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#trade-streams>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceTrade {
    #[serde(rename = "s", deserialize_with = "de_trade_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(rename = "T", deserialize_with = "crate::exchange::de_u64_epoch_ms_as_datetime_utc")]
    pub time: DateTime<Utc>,
    #[serde(rename = "a")]
    pub id: u64,
    #[serde(rename = "p", deserialize_with = "crate::exchange::de_str")]
    pub price: f64,
    #[serde(rename = "q", deserialize_with = "crate::exchange::de_str")]
    pub quantity: f64,
    #[serde(rename = "m", deserialize_with = "de_side_from_buyer_is_maker")]
    pub side: Side,
}

impl From<(ExchangeId, Instrument, BinanceTrade)> for MarketEvent {
    fn from((exchange_id, instrument, trade): (ExchangeId, Instrument, BinanceTrade)) -> Self {
        Self {
            exchange_time: trade.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::Trade(PublicTrade {
                id: trade.id.to_string(),
                price: trade.price,
                quantity: trade.quantity,
                side: trade.side
            }),
        }
    }
}

/// Todo:
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#level2-channel>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceOrderBookL2Update {
    #[serde(rename = "s", deserialize_with = "de_ob_l2_subscription_id")]
    pub subscription_id: SubscriptionId,

    // Todo: Must add the other fields for binance
    // Todo: deserialize_with = "level to level_delta"

    #[serde(alias = "b")]
    pub bids: Vec<LevelDelta>,
    #[serde(alias = "a")]
    pub asks: Vec<LevelDelta>,
}

impl From<(ExchangeId, Instrument, BinanceOrderBookL2Update)> for MarketEvent {
    fn from((exchange_id, instrument, ob_update): (ExchangeId, Instrument, BinanceOrderBookL2Update)) -> Self {
        Self {
            exchange_time: Utc::now(),
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::OrderBookDelta(OrderBookDelta {
                deltas: vec![], // Todo:
            }),
        }
    }
}

/// Deserialize a [`BinanceTrade`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`]
/// (eg/ "@aggTrade|BTCUSDT").
pub fn de_trade_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    serde::de::Deserialize::deserialize(deserializer)
        .map(|market| BinanceFuturesUsd::subscription_id(BinanceFuturesUsd::CHANNEL_TRADES, market))
}

/// Deserialize a [`BinanceTrade`] "buyer_is_maker" boolean field to a Barter [`Side`].
///
/// Variants:
/// buyer_is_maker => Side::Sell
/// !buyer_is_maker => Side::Buy
pub fn de_side_from_buyer_is_maker<'de, D>(deserializer: D) -> Result<Side, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    serde::de::Deserialize::deserialize(deserializer)
        .map(|buyer_is_maker| {
            if buyer_is_maker {
                Side::Sell
            } else {
                Side::Buy
            }
        })
}

/// Deserialize a [`BinanceOrderBookL2Update`] "s" (eg/ "BTCUSDT") as the associated
/// [`SubscriptionId`] (eg/ "@depth@100ms|BTCUSDT").
pub fn de_ob_l2_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
    where
        D: serde::de::Deserializer<'de>,
{
    serde::de::Deserialize::deserialize(deserializer)
        .map(|market| BinanceFuturesUsd::subscription_id(BinanceFuturesUsd::CHANNEL_ORDER_BOOK_L2, market))
}


#[cfg(test)]
mod tests {
    use serde::de::Error;
    use super::*;

    #[test]
    fn test_deserialise_binance_subscription_response() {
        struct TestCase {
            input: &'static str,
            expected: Result<BinanceSubResponse, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is Subscribed
                input: r#"{"id":1,"result":null}"#,
                expected: Ok(BinanceSubResponse {
                    result: None,
                    id: 1
                }),
            },
            TestCase {
                // TC1: input response is failed subscription
                input: r#"{"result": [], "id": 1}"#,
                expected: Ok(BinanceSubResponse {
                    result: Some(vec![]),
                    id: 1,
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<BinanceSubResponse>(test.input);
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
    fn test_validate_binance_subscription_response() {
        struct TestCase {
            input_response: BinanceSubResponse,
            is_valid: bool,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is successful subscription
                input_response: BinanceSubResponse {
                    result: None,
                    id: 1,
                },
                is_valid: true,
            },
            TestCase {
                // TC1: input response is failed subscription
                input_response: BinanceSubResponse {
                    result: Some(vec![]),
                    id: 1,
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
    fn test_deserialise_binance_message() {
        struct TestCase {
            input: &'static str,
            expected: Result<BinanceMessage, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: invalid BinanceMessage w/ unknown tag
                input: r#"{
                    "e":"unknown","E":1662494217187,"a":1048104319,"s":"ETHUSDT","p":"1575.96",
                    "q":"0.704","f":2189899361,"l":2189899363,"T":1662494217032,"m":false
                }"#,
                expected: Err(SocketError::Serde {
                    error: serde_json::Error::custom(""),
                    payload: "".to_owned(),
                }),
            },

            TestCase {
                // TC1: valid BinanceMessage Spot trade w/ Side::Sell
                input: r#"{
                    "e":"aggTrade","E":1662494217187,"a":1048104319,"s":"ETHUSDT","p":"1575.96",
                    "q":"0.704","f":2189899361,"l":2189899363,"T":1662494217032,"m":true
                }"#,
                expected: Ok(BinanceMessage::Trade(BinanceTrade {
                    subscription_id: SubscriptionId::from("@aggTrade|ETHUSDT"),
                    time: datetime_utc_from_epoch_duration(Duration::from_millis(1662494217032)),
                    id: 1048104319,
                    price: 1575.96,
                    quantity: 0.704,
                    side: Side::Sell,
                })),
            },
            TestCase {
                // TC2: valid BinanceMessage Spot trade w/ Side::Buy
                input: r#"{
                    "e":"aggTrade","E":1662494217187,"a":1048104319,"s":"ETHUSDT","p":"1575.96",
                    "q":"0.704","f":2189899361,"l":2189899363,"T":1662494217032,"m":false
                }"#,
                expected: Ok(BinanceMessage::Trade(BinanceTrade {
                    subscription_id: SubscriptionId::from("@aggTrade|ETHUSDT"),
                    time: datetime_utc_from_epoch_duration(Duration::from_millis(1662494217032)),
                    id: 1048104319,
                    price: 1575.96,
                    quantity: 0.704,
                    side: Side::Buy,
                })),
            },
            TestCase {
                // TC3: invalid BinanceMessage Spot trade w/ non-string price
                input: r#"{
                    "e":"aggTrade","E":1662494217187,"a":1048104319,"s":"ETHUSDT","p":1575.96,
                    "q":"0.704","f":2189899361,"l":2189899363,"T":1662494217032,"m":false
                }"#,
                expected: Err(SocketError::Serde {
                    error: serde_json::Error::custom(""),
                    payload: "".to_owned(),
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<BinanceMessage>(test.input);
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