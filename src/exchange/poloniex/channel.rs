use super::Poloniex;
use crate::{
    subscription::{trade::PublicTrades, Subscription},
    Identifier
};
use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Poloniex`](super::Poloniex) channel to be subscribed to.
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct PoloniexChannel(pub &'static str);

impl PoloniexChannel {
    /// [`Okx`] real-time trades channel.
    ///
    /// See docs: <https://docs.poloniex.com/#public-channels-market-data-trades>
    pub const TRADES: Self = Self("trades");
}

impl Identifier<PoloniexChannel> for Subscription<Poloniex, PublicTrades> {
    fn id(&self) -> PoloniexChannel {
        PoloniexChannel::TRADES
    }
}

impl AsRef<str> for PoloniexChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}