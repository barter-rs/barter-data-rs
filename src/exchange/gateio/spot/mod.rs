use self::trade::GateioSpotTrade;
use super::Gateio;
use crate::{
    exchange::{ExchangeId, ExchangeServer, StreamSelector},
    subscription::trade::PublicTrades,
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream,
};
use barter_macro::{DeExchange, SerExchange};

/// Public trades types.
pub mod trade;

/// [`GateioSpot`] WebSocket server base url.
///
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/>
pub const WEBSOCKET_BASE_URL_GATEIO_SPOT: &str = "wss://api.gateio.ws/ws/v4/";

/// [`Gateio`](super::Gateio) spot exchange.
pub type GateioSpot = Gateio<GateioServerSpot>;

/// [`Gateio`](super::Gateio) spot [`ExchangeServer`].
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, DeExchange, SerExchange,
)]
pub struct GateioServerSpot;

impl ExchangeServer for GateioServerSpot {
    const ID: ExchangeId = ExchangeId::GateioSpot;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_GATEIO_SPOT
    }
}

impl StreamSelector<PublicTrades> for GateioSpot {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, GateioSpotTrade>>;
}
