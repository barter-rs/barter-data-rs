use crate::{
    exchange::gateio::{
        futures::{GateioFuturesBtc, GateioFuturesUsd},
        spot::GateioSpot,
    },
    subscription::{trade::PublicTrades, Subscription},
    Identifier,
};
use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Gateio`](super::Gateio) channel to be subscribed to.
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct GateioChannel(pub &'static str);

impl GateioChannel {
    /// Gateio [`InstrumentKind::Spot`] real-time trades channel.
    ///
    /// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#public-trades-channel>
    pub const SPOT_TRADES: Self = Self("spot.trades");

    /// Gateio [`InstrumentKind::FuturePerpetual`] real-time trades channel.
    ///
    /// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#public-trades-channel>
    pub const FUTURE_PERPETUAL_TRADES: Self = Self("futures.trades");
}

impl Identifier<GateioChannel> for Subscription<GateioSpot, PublicTrades> {
    fn id(&self) -> GateioChannel {
        GateioChannel::SPOT_TRADES
    }
}

impl Identifier<GateioChannel> for Subscription<GateioFuturesUsd, PublicTrades> {
    fn id(&self) -> GateioChannel {
        GateioChannel::FUTURE_PERPETUAL_TRADES
    }
}

impl Identifier<GateioChannel> for Subscription<GateioFuturesBtc, PublicTrades> {
    fn id(&self) -> GateioChannel {
        GateioChannel::FUTURE_PERPETUAL_TRADES
    }
}

impl AsRef<str> for GateioChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
