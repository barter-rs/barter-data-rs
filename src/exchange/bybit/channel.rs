use serde::Serialize;
use crate::exchange::bybit::Bybit;
use crate::Identifier;
use crate::subscription::Subscription;
use crate::subscription::trade::PublicTrades;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct BybitChannel(pub &'static str);

impl BybitChannel {
    pub const TRADES: Self = Self("publicTrade");
}

impl<Server> Identifier<BybitChannel> for Subscription<Bybit<Server>, PublicTrades> {
    fn id(&self) -> BybitChannel {
        BybitChannel::TRADES
    }
}

impl AsRef<str> for BybitChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
