use super::Kraken;
use crate::{
    subscription::{trade::PublicTrades, Subscription},
    Identifier,
};
use serde::Serialize;

/// Todo:
///
/// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct KrakenChannel(pub &'static str);

impl KrakenChannel {
    /// [`Kraken`] real-time trades channel name.
    ///
    /// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
    const TRADES: Self = Self("trade");
}

impl Identifier<KrakenChannel> for Subscription<Kraken, PublicTrades> {
    fn id(&self) -> KrakenChannel {
        KrakenChannel::TRADES
    }
}

impl AsRef<str> for KrakenChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
