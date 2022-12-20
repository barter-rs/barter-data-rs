use super::Bitfinex;
use crate::{
    subscriber::subscription::{trade::PublicTrades, Subscription},
    Identifier,
};
use serde::Serialize;

/// Todo:
///
/// See docs: <https://docs.bitfinex.com/docs/ws-public>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct BitfinexChannel(pub &'static str);

impl BitfinexChannel {
    /// [`Bitfinex`] real-time trades channel.
    ///
    /// See docs: <https://docs.bitfinex.com/reference/ws-public-trades>
    pub const TRADES: Self = Self("trades");
}

impl Identifier<BitfinexChannel> for Subscription<Bitfinex, PublicTrades> {
    fn id(&self) -> BitfinexChannel {
        BitfinexChannel::TRADES
    }
}

impl AsRef<str> for BitfinexChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
