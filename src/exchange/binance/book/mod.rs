use crate::subscription::book::Level;
use serde::{Deserialize, Serialize};

/// Todo:
pub mod l1;
pub mod l2;

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

impl From<BinanceLevel> for Level {
    fn from(level: BinanceLevel) -> Self {
        Self {
            price: level.price,
            amount: level.amount,
        }
    }
}
