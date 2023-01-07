use crate::{
    event::{Market, MarketIter},
    exchange::{binance::channel::BinanceChannel, ExchangeId, subscription::ExchangeSub},
    Identifier,
    subscription::book::{Level, OrderBookL1},
};
use barter_integration::model::{Exchange, Instrument, SubscriptionId};
use chrono::Utc;
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

/// Deserialize a [`BinanceOrderBookL1`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`]
/// (eg/ "@bookTicker|BTCUSDT").
pub fn de_ob_l1_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((BinanceChannel::ORDER_BOOK_L1, market)).id())
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
}