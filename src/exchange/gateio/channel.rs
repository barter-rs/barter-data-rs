use crate::{
    subscription::{trade::PublicTrades, Subscription},
    Identifier,
};
use barter_integration::model::InstrumentKind;
use serde::Serialize;

/// Todo:
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct GateioChannel(pub &'static str);

impl GateioChannel {
    /// Gateio [`InstrumentKind::Spot`] real-time trades channel.
    ///
    /// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#public-trades-channel>
    const SPOT_TRADES: Self = Self("spot.trades");

    /// Gateio [`InstrumentKind::FuturePerpetual`] real-time trades channel.
    ///
    /// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#public-trades-channel>
    const FUTURE_PERPETUAL_TRADES: Self = Self("futures.trades");
}

impl<Server> Identifier<GateioChannel> for Subscription<Server, PublicTrades> {
    fn id(&self) -> GateioChannel {
        match self.instrument.kind {
            InstrumentKind::Spot => GateioChannel::SPOT_TRADES,
            InstrumentKind::FuturePerpetual => GateioChannel::FUTURE_PERPETUAL_TRADES,
        }
    }
}

impl AsRef<str> for GateioChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
