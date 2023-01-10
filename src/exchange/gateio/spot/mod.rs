use self::trade::GateioSpotTrade;
use super::{Gateio, GateioServer};
use crate::{
    exchange::ExchangeId, subscription::trade::PublicTrades,
    transformer::stateless::StatelessTransformer, ExchangeWsStream, StreamSelector,
};
use barter_macro::{DeExchange, SerExchange};

/// Todo:
pub mod trade;

/// [`GateioSpot`] WebSocket server base url.
///
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/>
pub const WEBSOCKET_BASE_URL_GATEIO_SPOT: &str = "wss://api.gateio.ws/ws/v4/";

/// Todo:
pub type GateioSpot = Gateio<GateioServerSpot>;

/// Todo:
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, DeExchange, SerExchange,
)]
pub struct GateioServerSpot;

impl GateioServer for GateioServerSpot {
    const ID: ExchangeId = ExchangeId::GateioSpot;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_GATEIO_SPOT
    }
}

impl StreamSelector<PublicTrades> for GateioSpot {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, GateioSpotTrade>>;
}
