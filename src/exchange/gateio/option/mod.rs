use crate::{
    exchange::{
        gateio::{perpetual::trade::GateioFuturesTrades, Gateio},
        ExchangeId, ExchangeServer, StreamSelector,
    },
    subscription::trade::PublicTrades,
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream,
};

/// [`GateioOptions`] WebSocket server base url.
///
/// See docs: <https://www.gate.io/docs/developers/futures/ws/en/>
pub const WEBSOCKET_BASE_URL_GATEIO_OPTIONS_USD: &str = "wss://op-ws.gateio.live/v4/ws";

/// [`Gateio`] options exchange.
pub type GateioOptions = Gateio<GateioServerOptions>;

/// [`Gateio`] options [`ExchangeServer`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct GateioServerOptions;

impl ExchangeServer for GateioServerOptions {
    const ID: ExchangeId = ExchangeId::GateioOptions;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_GATEIO_OPTIONS_USD
    }
}

impl StreamSelector<PublicTrades> for GateioOptions {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, GateioFuturesTrades>>;
}
