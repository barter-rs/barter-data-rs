use super::Coinbase;
use crate::{
    subscriber::subscription::{Subscription, trade::PublicTrades},
    Identifier,
};

/// Todo:
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Debug, Copy, Clone)]
pub struct CoinbaseChannel(pub &'static str);

impl AsRef<str> for CoinbaseChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl CoinbaseChannel {
    /// [`Coinbase`] real-time trades channel.
    ///
    /// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#match>
    pub const TRADES: Self = Self("matches");
}

impl Identifier<CoinbaseChannel> for Subscription<Coinbase, PublicTrades> {
    fn id(&self) -> CoinbaseChannel {
        CoinbaseChannel::TRADES
    }
}
