use super::futures::BinanceFuturesUsd;
use crate::model::{Level, Liquidation, MarkPrice, OrderBook};
use crate::{
    model::{DataKind, PublicTrade},
    ExchangeId, MarketEvent,
};
use barter_integration::{
    error::SocketError,
    model::{Exchange, Instrument, Side, SubscriptionId},
    Validator,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    OrderBookSnapshot(BinanceOrderBook),
    #[serde(alias = "forceOrder")]
    Liquidation(BinanceLiquidation),
    #[serde(alias = "markPriceUpdate")]
    MarkPrice(BinanceMarkPrice),
}

impl From<(ExchangeId, Instrument, BinanceMessage)> for MarketEvent {
    fn from((exchange, instrument, message): (ExchangeId, Instrument, BinanceMessage)) -> Self {
        match message {
            BinanceMessage::Trade(trade) => MarketEvent::from((exchange, instrument, trade)),
            BinanceMessage::OrderBookSnapshot(order_book) => {
                MarketEvent::from((exchange, instrument, order_book))
            }
            BinanceMessage::Liquidation(liquidation) => {
                MarketEvent::from((exchange, instrument, liquidation))
            }
            BinanceMessage::MarkPrice(mark_price) => {
                MarketEvent::from((exchange, instrument, mark_price))
            }
        }
    }
}

/// `Binance` real-time trade message.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#trade-streams>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceTrade {
    #[serde(alias = "s", deserialize_with = "de_trade_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(
        alias = "T",
        deserialize_with = "crate::exchange::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
    #[serde(alias = "a")]
    pub id: u64,
    #[serde(alias = "p", deserialize_with = "crate::exchange::de_str")]
    pub price: f64,
    #[serde(alias = "q", deserialize_with = "crate::exchange::de_str")]
    pub quantity: f64,
    #[serde(alias = "m", deserialize_with = "de_side_from_buyer_is_maker")]
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
                side: trade.side,
            }),
        }
    }
}

/// `Binance` OrderBook snapshot message.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#partial-book-depth-streams>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceOrderBook {
    #[serde(alias = "s", deserialize_with = "de_order_book_subscription_id")]
    pub subscription_id: SubscriptionId,

    #[serde(
        alias = "T",
        deserialize_with = "crate::exchange::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,

    #[serde(alias = "u")]
    pub last_update_id: u64,

    #[serde(alias = "b")]
    pub bids: Vec<BinanceLevel>,

    #[serde(alias = "a")]
    pub asks: Vec<BinanceLevel>,
}

/// `Binance` OrderBook level.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#partial-book-depth-streams>
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceLevel {
    #[serde(deserialize_with = "crate::exchange::de_str")]
    pub price: f64,
    #[serde(deserialize_with = "crate::exchange::de_str")]
    pub quantity: f64,
}

impl From<(ExchangeId, Instrument, BinanceOrderBook)> for MarketEvent {
    fn from(
        (exchange_id, instrument, snapshot): (ExchangeId, Instrument, BinanceOrderBook),
    ) -> Self {
        Self {
            exchange_time: snapshot.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::OrderBook(OrderBook {
                last_update_time: snapshot.time,
                last_update_id: snapshot.last_update_id,
                bids: snapshot.bids.into_iter().map(Level::from).collect(),
                asks: snapshot.asks.into_iter().map(Level::from).collect(),
            }),
        }
    }
}

impl From<BinanceLevel> for Level {
    fn from(level: BinanceLevel) -> Self {
        Self {
            price: level.price,
            quantity: level.quantity,
        }
    }
}

/// `Binance` Liquidation order message.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#liquidation-order-streams>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceLiquidation {
    #[serde(alias = "o")]
    pub order: BinanceLiquidationOrder,
}

impl From<(ExchangeId, Instrument, BinanceLiquidation)> for MarketEvent {
    fn from(
        (exchange_id, instrument, liquidation): (ExchangeId, Instrument, BinanceLiquidation),
    ) -> Self {
        Self {
            exchange_time: liquidation.order.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::Liquidation(Liquidation {
                side: liquidation.order.side,
                price: liquidation.order.price,
                quantity: liquidation.order.quantity,
                time: liquidation.order.time,
            }),
        }
    }
}

impl From<BinanceLiquidation> for Liquidation {
    fn from(liquidation: BinanceLiquidation) -> Self {
        Self {
            side: liquidation.order.side,
            price: liquidation.order.price,
            quantity: liquidation.order.quantity,
            time: liquidation.order.time,
        }
    }
}

/// `Binance` Liquidation order.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#liquidation-order-streams>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceLiquidationOrder {
    #[serde(alias = "s", deserialize_with = "de_liquidation_subscription_id")]
    pub subscription_id: SubscriptionId,

    #[serde(alias = "S")]
    pub side: Side,

    #[serde(alias = "p", deserialize_with = "crate::exchange::de_str")]
    pub price: f64,

    #[serde(alias = "q", deserialize_with = "crate::exchange::de_str")]
    pub quantity: f64,

    #[serde(
        alias = "T",
        deserialize_with = "crate::exchange::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
}

/// `Binance` mark price message.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#mark-price-stream>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceMarkPrice {
    #[serde(
        alias = "E",
        deserialize_with = "crate::exchange::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub event_time: DateTime<Utc>,
    #[serde(alias = "s", deserialize_with = "de_mark_price_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(alias = "p", deserialize_with = "crate::exchange::de_str")]
    pub mark_price: f64,
    #[serde(alias = "i", deserialize_with = "crate::exchange::de_str")]
    pub index_price: f64,
    #[serde(alias = "P", deserialize_with = "crate::exchange::de_str")]
    pub estimated_settlement_price: f64,
    #[serde(alias = "r", deserialize_with = "crate::exchange::de_str")]
    pub funding_rate: f64,
    #[serde(
        alias = "T",
        deserialize_with = "crate::exchange::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub next_funding_time: DateTime<Utc>,
}

impl From<(ExchangeId, Instrument, BinanceMarkPrice)> for MarketEvent {
    fn from(
        (exchange_id, instrument, mark_price): (ExchangeId, Instrument, BinanceMarkPrice),
    ) -> Self {
        Self {
            exchange_time: mark_price.event_time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::MarkPrice(MarkPrice {
                mark_price: mark_price.mark_price,
                index_price: mark_price.index_price,
                estimated_settlement_price: mark_price.estimated_settlement_price,
                funding_rate: mark_price.funding_rate,
                next_funding_time: mark_price.next_funding_time,
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
    serde::de::Deserialize::deserialize(deserializer).map(|buyer_is_maker| {
        if buyer_is_maker {
            Side::Sell
        } else {
            Side::Buy
        }
    })
}

/// Deserialize a [`BinanceOrderBook`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`]
/// (eg/ "@depth@20@100ms|BTCUSDT").
pub fn de_order_book_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    serde::de::Deserialize::deserialize(deserializer).map(|market| {
        BinanceFuturesUsd::subscription_id(BinanceFuturesUsd::CHANNEL_ORDER_BOOK, market)
    })
}

/// Deserialize a [`BinanceLiquidationOrder`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`]
/// (eg/ "forceOrder|BTCUSDT").
pub fn de_liquidation_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    serde::de::Deserialize::deserialize(deserializer).map(|market| {
        BinanceFuturesUsd::subscription_id(BinanceFuturesUsd::CHANNEL_LIQUIDATIONS, market)
    })
}

/// Deserialize a [`BinanceMarkPrice`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`]
/// (eg/ "@markPrice@1s|BTCUSDT").
pub fn de_mark_price_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    serde::de::Deserialize::deserialize(deserializer).map(|market| {
        BinanceFuturesUsd::subscription_id(BinanceFuturesUsd::CHANNEL_MARK_PRICE, market)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange::datetime_utc_from_epoch_duration;
    use serde::de::Error;
    use std::time::Duration;

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
                    id: 1,
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
                expected: Err(SocketError::Deserialise {
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
                expected: Err(SocketError::Deserialise {
                    error: serde_json::Error::custom(""),
                    payload: "".to_owned(),
                }),
            },
            TestCase {
                // TC4: valid BinanceMessage FuturePerpetualUsd Liquidation
                input: r#"{
                    "e": "forceOrder",
                    "E": 1665523974222,
                    "o": {
                      "s": "BTCUSDT",
                      "S": "SELL",
                      "o": "LIMIT",
                      "f": "IOC",
                      "q": "0.009",
                      "p": "18917.15",
                      "ap": "18990.00",
                      "X": "FILLED",
                      "l": "0.009",
                      "z": "0.009",
                      "T": 1665523974217
                    }
                  }"#,
                expected: Ok(BinanceMessage::Liquidation(BinanceLiquidation {
                    order: BinanceLiquidationOrder {
                        subscription_id: SubscriptionId::from("@forceOrder|BTCUSDT"),
                        side: Side::Sell,
                        price: 18917.15,
                        quantity: 0.009,
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1665523974217,
                        )),
                    },
                })),
            },
            TestCase {
                // TC5: valid BinanceMessage FuturePerpetualUsd Mark Price
                input: r#"  {
                    "e": "markPriceUpdate",
                    "E": 1562305380000,
                    "s": "BTCUSDT",
                    "p": "11794.15000000",
                    "i": "11784.62659091",
                    "P": "11784.25641265",
                    "r": "0.00038167",
                    "T": 1562306400000
                  }"#,
                expected: Ok(BinanceMessage::MarkPrice(BinanceMarkPrice {
                    event_time: datetime_utc_from_epoch_duration(Duration::from_millis(
                        1562305380000,
                    )),
                    subscription_id: SubscriptionId::from("@markPrice@1s|BTCUSDT"),
                    mark_price: 11794.15000000,
                    index_price: 11784.62659091,
                    estimated_settlement_price: 11784.25641265,
                    funding_rate: 0.00038167,
                    next_funding_time: datetime_utc_from_epoch_duration(Duration::from_millis(
                        1562306400000,
                    )),
                })),
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
