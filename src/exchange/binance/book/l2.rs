use crate::{
    exchange::{binance::channel::BinanceChannel, subscription::ExchangeSub},
    subscription::book::{OrderBook, Level},
    Identifier,
};
use barter_integration::model::SubscriptionId;
use serde::{Deserialize, Serialize};
use chrono::Utc;

/// [`Binance`](super::Binance) OrderBook Level2 snapshot HTTP message.
///
/// Used as the starting [`OrderBook`] before [`BinanceOrderBookL2Delta`] WebSocket updates are
/// applied.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#order-book>
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#order-book>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceOrderBookL2Snapshot {
    #[serde(rename = "lastUpdateId")]
    last_update_id: u64,
    bids: Vec<Level>,
    asks: Vec<Level>,
}

impl From<BinanceOrderBookL2Snapshot> for OrderBook {
    fn from(snapshot: BinanceOrderBookL2Snapshot) -> Self {
        Self {
            last_update_time: Utc::now(),
            last_update_id: snapshot.last_update_id,
            bids: snapshot.bids,
            asks: snapshot.asks
        }
    }
}

/// [`Binance`](super::Binance) OrderBook Level2 deltas WebSocket message.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#partial-book-depth-streams>
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#partial-book-depth-streams>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceOrderBookL2Delta {
    #[serde(alias = "s", deserialize_with = "de_ob_l2_subscription_id")]
    pub subscription_id: SubscriptionId,

    #[serde(alias = "U")]
    pub first_update_id: u64,

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

impl Identifier<Option<SubscriptionId>> for BinanceOrderBookL2Delta {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

// impl From<(ExchangeId, Instrument, BinanceOrderBookL2Delta)> for MarketIter<OrderBook> {
//     fn from((exchange_id, instrument, book): (ExchangeId, Instrument, BinanceOrderBookL2Delta)) -> Self {
//         let time = Utc::now();
//
//         Self(vec![Ok(Market {
//             exchange_time: time,
//             received_time: Utc::now(),
//             exchange: Exchange::from(exchange_id),
//             instrument,
//             event: OrderBook {
//                 last_update_time: time,
//                 last_update_id: book.last_update_id,
//                 bids: book.bids.into_iter().map(Level::from).collect(),
//                 asks: book.asks.into_iter().map(Level::from).collect(),
//             },
//         })])
//     }
// }

impl From<BinanceLevel> for Level {
    fn from(level: BinanceLevel) -> Self {
        Self {
            price: level.price,
            amount: level.amount,
        }
    }
}

/// Deserialize a [`BinanceOrderBookL2`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`]
/// (eg/ "@depth20@100ms|BTCUSDT").
pub fn de_ob_l2_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((BinanceChannel::ORDER_BOOK_L2, market)).id())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_de_binance_order_book_l2_deltas() {
        struct TestCase {
            input: &'static str,
            expected: BinanceOrderBookL2Delta,
        }

        let tests = vec![
            TestCase {
                // TC0: valid Spot BinanceOrderBookL2Delta
                input: r#"
                {
                    "e":"depthUpdate",
                    "E":1671656397761,
                    "s":"ETHUSDT",
                    "U":22611425143,
                    "u":22611425151,
                    "b":[
                        ["1209.67000000","85.48210000"],
                        ["1209.66000000","20.68790000"]
                    ],
                    "a":[]
                }
                "#,
                expected: BinanceOrderBookL2Delta {
                    subscription_id: SubscriptionId::from("@depth@100ms|ETHUSDT"),
                    first_update_id: 22611425143,
                    last_update_id: 22611425154,
                    bids: vec![
                        BinanceLevel { price: 1209.67000000, amount: 85.48210000 },
                        BinanceLevel { price: 1209.66000000, amount: 20.68790000 },
                    ],
                    asks: vec![]
                },
            },
            TestCase {
                // TC1: valid FuturePerpetual BinanceOrderBookL2Delta
                input: r#"
                {
                }"#,
                expected: BinanceOrderBookL2Delta {
                    subscription_id: SubscriptionId::from("@depth@100ms|BTCUSDT"),
                    first_update_id: 0,
                    last_update_id: 0,
                    bids: vec![],
                    asks: vec![]
                },
            },
        ];
    }
}