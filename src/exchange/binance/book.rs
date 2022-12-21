use crate::{
    event::{Market, MarketIter},
    exchange::{binance::channel::BinanceChannel, subscription::ExchangeSub, ExchangeId},
    subscription::book::{OrderBookL1, OrderBook, Level},
    Identifier,
};
use barter_integration::model::{Exchange, Instrument, SubscriptionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// [`Binance`](super::Binance) real-time OrderBook Level1 (top of book) message.
///
/// See docs:<https://binance-docs.github.io/apidocs/spot/en/#individual-symbol-book-ticker-streams>
/// See docs:<https://binance-docs.github.io/apidocs/futures/en/#individual-symbol-book-ticker-streams>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceOrderBookL1 {
    #[serde(alias = "s", deserialize_with = "de_ob_l1_subscription_id")]
    subscription_id: SubscriptionId,
    #[serde(alias = "u")]
    last_update_id: u64,
    #[serde(alias = "b", deserialize_with = "barter_integration::de::de_str")]
    best_bid_price: f64,
    #[serde(alias = "B", deserialize_with = "barter_integration::de::de_str")]
    best_bid_amount: f64,
    #[serde(alias = "a", deserialize_with = "barter_integration::de::de_str")]
    best_ask_price: f64,
    #[serde(alias = "A", deserialize_with = "barter_integration::de::de_str")]
    best_ask_amount: f64,
}

impl Identifier<Option<SubscriptionId>> for BinanceOrderBookL1 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl From<(ExchangeId, Instrument, BinanceOrderBookL1)> for MarketIter<OrderBookL1> {
    fn from((exchange_id, instrument, book): (ExchangeId, Instrument, BinanceOrderBookL1)) -> Self {
        // Create timestamp to be used for all required time fields that are not present
        let time_now = Utc::now();

        Self(vec![Ok(Market {
            exchange_time: time_now,
            received_time: time_now,
            exchange: Exchange::from(exchange_id),
            instrument,
            event: OrderBookL1 {
                last_update_time: time_now,
                last_update_id: book.last_update_id,
                best_bid: Level::new(book.best_bid_price, book.best_bid_amount),
                best_ask: Level::new(book.best_ask_price, book.best_ask_amount),
            },
        })])
    }
}

/// [`Binance`](super::Binance) OrderBook Level2 snapshot message.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#partial-book-depth-streams>
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#partial-book-depth-streams>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceOrderBookL2 {
    #[serde(alias = "s", deserialize_with = "de_ob_l2_subscription_id")]
    pub subscription_id: SubscriptionId,

    #[serde(alias = "T", deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc")]
    pub time: DateTime<Utc>,

    #[serde(alias = "u")]
    pub last_update_id: u64,

    #[serde(alias = "b")]
    pub bids: Vec<BinanceLevel>,

    #[serde(alias = "a")]
    pub asks: Vec<BinanceLevel>,
}

/// [`Binance`](super::Binance) OrderBook level.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#partial-book-depth-streams>
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceLevel {
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub amount: f64,
}

impl Identifier<Option<SubscriptionId>> for BinanceOrderBookL2 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl From<(ExchangeId, Instrument, BinanceOrderBookL2)> for MarketIter<OrderBook> {
    fn from((exchange_id, instrument, book): (ExchangeId, Instrument, BinanceOrderBookL2)) -> Self {
        Self(vec![Ok(Market {
            exchange_time: book.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            event: OrderBook {
                last_update_time: book.time,
                last_update_id: book.last_update_id,
                bids: book.bids.into_iter().map(Level::from).collect(),
                asks: book.asks.into_iter().map(Level::from).collect(),
            },
        })])
    }
}

impl From<BinanceLevel> for Level {
    fn from(level: BinanceLevel) -> Self {
        Self {
            price: level.price,
            amount: level.amount,
        }
    }
}

/// Deserialize a [`BinanceOrderBookL1`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`]
/// (eg/ "@bookTicker|BTCUSDT").
pub fn de_ob_l1_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((BinanceChannel::ORDER_BOOK_L1, market)).id())
}

/// Deserialize a [`BinanceOrderBookL2`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`]
/// (eg/ "@depth20@100ms|BTCUSDT").
pub fn de_ob_l2_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((BinanceChannel::ORDER_BOOK_L2_SNAPSHOT, market)).id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_de_binance_order_book_l1() {
        struct TestCase {
            input: &'static str,
            expected: BinanceOrderBookL1,
        }

        let tests = vec![
            TestCase {
                // TC0: valid Spot BinanceOrderBookL1
                input: r#"{
                    "u":22606535573,
                    "s":"ETHUSDT",
                    "b":"1215.27000000",
                    "B":"32.49110000",
                    "a":"1215.28000000",
                    "A":"13.93900000"
                }
                "#,
                expected: BinanceOrderBookL1 {
                    subscription_id: SubscriptionId::from("@bookTicker|ETHUSDT"),
                    last_update_id: 22606535573,
                    best_bid_price: 1215.27000000,
                    best_bid_amount: 32.49110000,
                    best_ask_price: 1215.28000000,
                    best_ask_amount: 13.93900000,
                },
            },
            TestCase {
                // TC1: valid FuturePerpetual BinanceOrderBookL1
                input: r#"{
                    "e":"bookTicker",
                    "u":2286618712950,
                    "s":"BTCUSDT",
                    "b":"16858.90",
                    "B":"13.692",
                    "a":"16859.00",
                    "A":"30.219",
                    "T":1671621244670,
                    "E":1671621244673
                }"#,
                expected: BinanceOrderBookL1 {
                    subscription_id: SubscriptionId::from("@bookTicker|BTCUSDT"),
                    last_update_id: 2286618712950,
                    best_bid_price: 16858.90,
                    best_bid_amount: 13.692,
                    best_ask_price: 16859.00,
                    best_ask_amount: 30.219,
                },
            },
        ];
    }

    #[test]
    fn test_de_binance_order_book_l2() {
        struct TestCase {
            input: &'static str,
            expected: BinanceOrderBookL1,
        }

        let tests = vec![
            TestCase {
                // TC0: valid Spot BinanceOrderBookL1
                input: r#"
                {
                    "lastUpdateId":22611319997,
                    "bids":[
                        ["1209.15000000","13.67910000"],
                        ["1209.13000000","0.22000000"],
                        ["1209.12000000","17.53170000"],
                    ],
                    "asks":[
                        ["1209.16000000","122.74250000"],
                        ["1209.17000000","2.11820000"],
                        ["1209.18000000","19.67060000"],
                    ]
                }
                "#,
                expected: BinanceOrderBookL2 {
                    subscription_id: SubscriptionId::from("@depth20@100ms|ETHUSDT"),
                    time: ,
                    last_update_id: 0,
                    bids: vec![],
                    asks: vec![]
                },
            },
            TestCase {
                // TC1: valid FuturePerpetual BinanceOrderBookL1
                input: r#"{
                    "e":"bookTicker",
                    "u":2286618712950,
                    "s":"BTCUSDT",
                    "b":"16858.90",
                    "B":"13.692",
                    "a":"16859.00",
                    "A":"30.219",
                    "T":1671621244670,
                    "E":1671621244673
                }"#,
                expected: BinanceOrderBookL1 {
                    subscription_id: SubscriptionId::from("@bookTicker|BTCUSDT"),
                    last_update_id: 2286618712950,
                    best_bid_price: 16858.90,
                    best_bid_amount: 13.692,
                    best_ask_price: 16859.00,
                    best_ask_amount: 30.219,
                },
            },
        ];
    }
}
