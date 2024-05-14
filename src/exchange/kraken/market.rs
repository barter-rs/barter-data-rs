use super::Kraken;
use crate::instrument::MarketInstrumentData;
use crate::{subscription::Subscription, Identifier};
use barter_integration::model::instrument::Instrument;
use serde::{Deserialize, Serialize};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Kraken`] market that can be subscribed to.
///
/// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct KrakenMarket(pub String);

impl<Kind> Identifier<KrakenMarket> for Subscription<Kraken, Instrument, Kind> {
    fn id(&self) -> KrakenMarket {
        KrakenMarket(format!("{}/{}", self.instrument.base, self.instrument.quote).to_uppercase())
    }
}

impl<Kind> Identifier<KrakenMarket> for Subscription<Kraken, MarketInstrumentData, Kind> {
    fn id(&self) -> KrakenMarket {
        KrakenMarket(self.instrument.name_exchange.clone())
    }
}

impl AsRef<str> for KrakenMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
