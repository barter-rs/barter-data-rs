use super::Bitfinex;
use crate::{subscription::Subscription, Identifier};
use serde::{Deserialize, Serialize};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Bitfinex`](super::Bitfinex) market that can be subscribed to.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-public>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BitfinexMarket(pub String);

impl<Kind> Identifier<BitfinexMarket> for Subscription<Bitfinex, Kind> {
    fn id(&self) -> BitfinexMarket {
        BitfinexMarket(format!(
            "t{}{}",
            self.instrument.base.to_string().to_uppercase(),
            self.instrument.quote.to_string().to_uppercase()
        ))
    }
}

impl AsRef<str> for BitfinexMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
