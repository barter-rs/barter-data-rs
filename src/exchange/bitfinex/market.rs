use super::Bitfinex;
use crate::instrument::MarketInstrumentData;
use crate::{subscription::Subscription, Identifier};
use barter_integration::model::instrument::Instrument;
use serde::{Deserialize, Serialize};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Bitfinex`] market that can be subscribed to.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-public>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BitfinexMarket(pub String);

impl<Kind> Identifier<BitfinexMarket> for Subscription<Bitfinex, Instrument, Kind> {
    fn id(&self) -> BitfinexMarket {
        BitfinexMarket(format!(
            "t{}{}",
            self.instrument.base.to_string().to_uppercase(),
            self.instrument.quote.to_string().to_uppercase()
        ))
    }
}

impl<Kind> Identifier<BitfinexMarket> for Subscription<Bitfinex, MarketInstrumentData, Kind> {
    fn id(&self) -> BitfinexMarket {
        BitfinexMarket(self.instrument.name_exchange.clone())
    }
}

impl AsRef<str> for BitfinexMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
