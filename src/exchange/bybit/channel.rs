use serde::Serialize;
use crate::exchange::bybit::Bybit;
use crate::Identifier;
use crate::subscription::Subscription;
use crate::subscription::trade::PublicTrades;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]

/// Type that defines how to translate a Barter [`Subscription`] into a [`Bybit`](super::Bybit)
/// channel to be subscribed to.
///
/// See docs: <https://bybit-exchange.github.io/docs/v5/ws/connect>
pub struct BybitChannel(pub &'static str);

impl BybitChannel {
    /// [`Binance`](super::Bybit) real-time trades channel name.
    ///
    /// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/trade>
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
