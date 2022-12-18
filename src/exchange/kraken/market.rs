use super::Kraken;
use crate::{
    subscriber::subscription::Subscription,
    Identifier,
};
use serde::{Deserialize, Serialize};

/// Todo:
///
/// See docs: <>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct KrakenMarket(pub String);

impl<Kind> Identifier<KrakenMarket> for Subscription<Kraken, Kind> {
    fn id(&self) -> KrakenMarket {
        KrakenMarket(format!("{}/{}", self.instrument.base, self.instrument.quote).to_uppercase())
    }
}

impl AsRef<str> for KrakenMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}