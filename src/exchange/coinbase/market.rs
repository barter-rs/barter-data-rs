use super::Coinbase;
use crate::{subscriber::subscription::Subscription, Identifier};
use serde::{Deserialize, Serialize};

/// Todo:
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct CoinbaseMarket(pub String);

impl<Kind> Identifier<CoinbaseMarket> for Subscription<Coinbase, Kind> {
    fn id(&self) -> CoinbaseMarket {
        CoinbaseMarket(format!("{}-{}", self.instrument.base, self.instrument.quote).to_uppercase())
    }
}

impl AsRef<str> for CoinbaseMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
