use super::Okx;
use crate::{
    subscriber::subscription::Subscription,
    Identifier
};
use barter_integration::model::InstrumentKind;
use serde::{Deserialize, Serialize};

/// Todo:
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OkxMarket(pub String);

impl<Kind> Identifier<OkxMarket> for Subscription<Okx, Kind> {
    fn id(&self) -> OkxMarket {
        OkxMarket(match self.instrument.kind {
            InstrumentKind::Spot => {
                format!("{}-{}", self.instrument.base, self.instrument.quote).to_uppercase()
            }
            InstrumentKind::FuturePerpetual => {
                format!("{}-{}-SWAP", self.instrument.base, self.instrument.quote).to_uppercase()
            }
        })
    }
}

impl AsRef<str> for OkxMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
