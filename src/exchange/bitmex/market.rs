use crate::instrument::MarketInstrumentData;
use crate::{exchange::bitmex::Bitmex, subscription::Subscription, Identifier};
use barter_integration::model::instrument::Instrument;
use serde::{Deserialize, Serialize};

/// Type that defines how to translate a Barter [`Subscription`] into a [`Bitmex`]
/// market that can be subscribed to.
///
/// See docs: <https://www.bitmex.com/app/wsAPI>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BitmexMarket(pub String);

impl<Kind> Identifier<BitmexMarket> for Subscription<Bitmex, Instrument, Kind> {
    fn id(&self) -> BitmexMarket {
        // Notes:
        // - Must be uppercase since Bitmex sends message with uppercase MARKET (eg/ XBTUSD).
        BitmexMarket(format!("{}{}", self.instrument.base, self.instrument.quote).to_uppercase())
    }
}

impl<Kind> Identifier<BitmexMarket> for Subscription<Bitmex, MarketInstrumentData, Kind> {
    fn id(&self) -> BitmexMarket {
        BitmexMarket(self.instrument.name_exchange.clone())
    }
}

impl AsRef<str> for BitmexMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
