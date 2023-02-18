use crate::{
    exchange::bybit::Bybit,
    subscription::{trade::PublicTrades, Subscription},
    Identifier,
};
use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a [`Bybit`](super::Bybit)
/// channel to be subscribed to.
///
/// See docs: <https://bybit-exchange.github.io/docs/v5/ws/connect>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct BitmexChannel(pub &'static str);

impl BitmexChannel {
    /// [`Bybit`](super::Bybit) real-time trades channel name.
    ///
    /// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/trade>
    pub const TRADES: Self = Self("trade");
}

impl<Server> Identifier<BitmexChannel> for Subscription<Bybit<Server>, PublicTrades> {
    fn id(&self) -> BitmexChannel {
        BitmexChannel::TRADES
    }
}

impl AsRef<str> for BitmexChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
