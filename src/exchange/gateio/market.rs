use super::Gateio;
use crate::{subscription::Subscription, Identifier};
use serde::{Deserialize, Serialize};

/// Todo:
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct GateioMarket(pub String);

impl<Server, Kind> Identifier<GateioMarket> for Subscription<Gateio<Server>, Kind> {
    fn id(&self) -> GateioMarket {
        GateioMarket(format!("{}_{}", self.instrument.base, self.instrument.quote).to_uppercase())
    }
}

impl AsRef<str> for GateioMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
