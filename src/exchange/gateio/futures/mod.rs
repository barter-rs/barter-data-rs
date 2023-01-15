use self::trade::GateioFuturesTrades;
use super::Gateio;
use crate::{
    exchange::{ExchangeId, ExchangeServer, StreamSelector},
    subscription::trade::PublicTrades,
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream,
};
use serde::{Deserialize, Serialize};

/// Public trades types.
pub mod trade;

/// [`GateioFuturesUsd`] WebSocket server base url.
///
/// See docs: <https://www.gate.io/docs/developers/futures/ws/en/>
pub const WEBSOCKET_BASE_URL_GATEIO_FUTURES_USD: &str = "wss://fx-ws.gateio.ws/v4/ws/usdt";

/// [`Gateio`](super::Gateio) futures usd exchange.
pub type GateioFuturesUsd = Gateio<GateioServerFuturesUsd>;

/// [`Gateio`](super::Gateio) futures usd [`ExchangeServer`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct GateioServerFuturesUsd;

impl ExchangeServer for GateioServerFuturesUsd {
    const ID: ExchangeId = ExchangeId::GateioFuturesUsd;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_GATEIO_FUTURES_USD
    }
}

impl StreamSelector<PublicTrades> for GateioFuturesUsd {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, GateioFuturesTrades>>;
}

/// [`GateioFuturesBtc`] WebSocket server base url.
///
/// See docs: <https://www.gate.io/docs/developers/futures/ws/en/>
pub const WEBSOCKET_BASE_URL_GATEIO_FUTURES_BTC: &str = "wss://fx-ws.gateio.ws/v4/ws/btc";

/// [`Gateio`](super::Gateio) futures btc exchange.
pub type GateioFuturesBtc = Gateio<GateioServerFuturesBtc>;

/// [`Gateio`](super::Gateio) futures btc [`ExchangeServer`].
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize,
)]
pub struct GateioServerFuturesBtc;

impl ExchangeServer for GateioServerFuturesBtc {
    const ID: ExchangeId = ExchangeId::GateioFuturesBtc;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_GATEIO_FUTURES_BTC
    }
}

impl StreamSelector<PublicTrades> for GateioFuturesBtc {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, GateioFuturesTrades>>;
}
