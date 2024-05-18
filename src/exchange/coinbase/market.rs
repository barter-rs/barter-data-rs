use super::Coinbase;
use crate::instrument::MarketInstrumentData;
use crate::{subscription::Subscription, Identifier};
use barter_integration::model::instrument::Instrument;
use serde::{Deserialize, Serialize};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Coinbase`](super::Coinbase) market that can be subscribed to.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct CoinbaseMarket(pub String);

impl<Kind> Identifier<CoinbaseMarket> for Subscription<Coinbase, Instrument, Kind> {
    fn id(&self) -> CoinbaseMarket {
        CoinbaseMarket(format!("{}-{}", self.instrument.base, self.instrument.quote).to_uppercase())
    }
}

impl<Kind> Identifier<CoinbaseMarket> for Subscription<Coinbase, MarketInstrumentData, Kind> {
    fn id(&self) -> CoinbaseMarket {
        CoinbaseMarket(self.instrument.name_exchange.clone())
    }
}

impl AsRef<str> for CoinbaseMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
