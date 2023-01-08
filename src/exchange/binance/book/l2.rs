use super::super::channel::BinanceChannel;
use super::BinanceLevel;
use crate::{
    exchange::subscription::ExchangeSub,
    subscription::book::{OrderBook, OrderBookSide},
    Identifier,
};
use barter_integration::model::{Side, SubscriptionId};
use serde::{Deserialize, Serialize};
use chrono::Utc;

/// [`Binance`](super::Binance) OrderBook Level2 snapshot HTTP message.
///
/// Used as the starting [`OrderBook`] before OrderBook Level2 delta WebSocket updates are
/// applied.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#order-book>
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#order-book>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceOrderBookL2Snapshot {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: u64,
    pub bids: Vec<BinanceLevel>,
    pub asks: Vec<BinanceLevel>,
}

impl From<BinanceOrderBookL2Snapshot> for OrderBook {
    fn from(snapshot: BinanceOrderBookL2Snapshot) -> Self {
        Self {
            last_update_time: Utc::now(),
            bids: OrderBookSide::new(Side::Buy, snapshot.bids),
            asks: OrderBookSide::new(Side::Sell, snapshot.asks),
        }
    }
}

/// Deserialize a [`BinanceOrderBookL2`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`]
/// (eg/ "@depth@100ms|BTCUSDT").
pub fn de_ob_l2_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((BinanceChannel::ORDER_BOOK_L2, market)).id())
}

#[cfg(test)]
mod tests {
    use crate::exchange::binance::futures::l2::BinanceFuturesOrderBookL2Delta;
    use super::*;

    #[test]
    fn test_de_binance_order_book_l2_deltas() {
        struct TestCase {
            input: &'static str,
            expected: BinanceFuturesOrderBookL2Delta,
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
                expected: BinanceFuturesOrderBookL2Delta {
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
                expected: BinanceFuturesOrderBookL2Delta {
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