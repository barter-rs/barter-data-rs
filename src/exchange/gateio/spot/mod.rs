use super::{spot::trade::GateioSpotTrade, Gateio};
use crate::{
    exchange::{ExchangeId, ServerSelector},
    subscription::trade::PublicTrades,
    transformer::StatelessTransformer,
    ExchangeWsStream, StreamSelector,
};
use serde::{Deserialize, Serialize};

/// Todo:
pub mod trade;

/// [`GateioSpot`] server base url.
///
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/>
pub const BASE_URL_GATEIO_SPOT: &str = "wss://api.gateio.ws/ws/v4/";

/// Todo:
pub type GateioSpot = Gateio<GateioServerSpot>;

/// Todo:
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize,
)]
pub struct GateioServerSpot;

impl ServerSelector for GateioServerSpot {
    const ID: ExchangeId = ExchangeId::GateioSpot;

    fn base_url() -> &'static str {
        BASE_URL_GATEIO_SPOT
    }
}

impl StreamSelector<PublicTrades> for GateioSpot {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, GateioSpotTrade>>;
}
