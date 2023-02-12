use serde::{Deserialize, Serialize};
use crate::exchange::bybit::Bybit;
use crate::Identifier;
use crate::subscription::Subscription;

/// Type that defines how to translate a Barter [`Subscription`] into a [`Bybit`](super::Bybit)
/// market that can be subscribed to.
///
/// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/trade>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BybitMarket(pub String);

impl<Server, Kind> Identifier<BybitMarket> for Subscription<Bybit<Server>, Kind> {
    fn id(&self) -> BybitMarket {
        // Notes:
        // - Must be uppercase since Bybit sends message with uppercase MARKET (eg/ BTCUSDT).
        BybitMarket(format!("{}{}", self.instrument.base, self.instrument.quote).to_uppercase())
    }
}

impl AsRef<str> for BybitMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
