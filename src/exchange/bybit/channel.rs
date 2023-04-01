use crate::{
    exchange::bybit::Bybit,
    subscription::{liquidation::Liquidations, trade::PublicTrades, Subscription},
    Identifier,
};
use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a [`Bybit`](super::Bybit)
/// channel to be subscribed to.
///
/// See docs: <https://bybit-exchange.github.io/docs/v5/ws/connect>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct BybitChannel(pub &'static str);

impl BybitChannel {
    /// [`Bybit`](super::Bybit) real-time trades channel.
    ///
    /// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/trade>
    pub const TRADES: Self = Self("publicTrade");

    /// [`Bybit`](super::Bybit) real-time liquidations channel.
    ///
    /// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/liquidation>
    pub const LIQUIDATIONS: Self = Self("liquidation");
}

impl<Server> Identifier<BybitChannel> for Subscription<Bybit<Server>, PublicTrades> {
    fn id(&self) -> BybitChannel {
        BybitChannel::TRADES
    }
}

impl<Server> Identifier<BybitChannel> for Subscription<Bybit<Server>, Liquidations> {
    fn id(&self) -> BybitChannel {
        BybitChannel::LIQUIDATIONS
    }
}

impl AsRef<str> for BybitChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
