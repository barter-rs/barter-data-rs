use crate::exchange::coinbase::Coinbase;
use crate::subscriber::subscription::Subscription;
use crate::Identifier;

/// Todo:
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Debug, Clone)]
pub struct CoinbaseMarket(pub String);

impl AsRef<str> for CoinbaseMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<Kind> Identifier<CoinbaseMarket> for Subscription<Coinbase, Kind> {
    fn id(&self) -> CoinbaseMarket {
        CoinbaseMarket(format!("{}-{}", self.instrument.base, self.instrument.quote).to_uppercase())
    }
}
