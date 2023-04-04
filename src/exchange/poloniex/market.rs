use super::Poloniex;
use crate::{subscription::Subscription, Identifier};
use serde::{Deserialize, Serialize};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Poloniex`](super::Poloniex) channel to be subscribed to.
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct PoloniexMarket(pub String);

impl <Kind> Identifier<PoloniexMarket> for Subscription<Poloniex, Kind>{
    fn id(&self) -> PoloniexMarket {
        PoloniexMarket(format!("{}_{}", self.instrument.base, self.instrument.quote).to_uppercase())
    }
}

impl AsRef<str> for PoloniexMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}