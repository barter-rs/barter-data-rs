use super::Binance;
use crate::{subscriber::subscription::Subscription, Identifier};
use serde::{Deserialize, Serialize};

/// Todo:
///
/// See docs: <>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BinanceMarket(pub String);

impl<Server, Kind> Identifier<BinanceMarket> for Subscription<Binance<Server>, Kind> {
    fn id(&self) -> BinanceMarket {
        // Notes:
        // - Must be lowercase when subscribing (transformed to lowercase by Binance fn requests).
        // - Must be uppercase since Binance sends message with uppercase MARKET (eg/ BTCUSDT).
        BinanceMarket(format!("{}{}", self.instrument.base, self.instrument.quote).to_uppercase())
    }
}

impl AsRef<str> for BinanceMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
