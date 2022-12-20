use super::{futures::trade::GateioFuturesTrades, Gateio};
use crate::{
    exchange::{ExchangeId, ServerSelector},
    ExchangeWsStream,
    StreamSelector, subscription::trade::PublicTrades,
};
use serde::{Deserialize, Serialize};
use crate::transformer::stateless::StatelessTransformer;

/// Todo:
pub mod trade;

/// [`GateioFuturesUsd`] server base url.
///
/// See docs: <https://www.gate.io/docs/developers/futures/ws/en/>
pub const BASE_URL_GATEIO_FUTURES_USD: &str = "wss://fx-ws.gateio.ws/v4/ws/usdt";

/// Todo:
pub type GateioFuturesUsd = Gateio<GateioServerFuturesUsd>;

/// Todo:
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize,
)]
pub struct GateioServerFuturesUsd;

impl ServerSelector for GateioServerFuturesUsd {
    const ID: ExchangeId = ExchangeId::GateioFuturesUsd;

    fn base_url() -> &'static str {
        BASE_URL_GATEIO_FUTURES_USD
    }
}

impl StreamSelector<PublicTrades> for GateioFuturesUsd {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, GateioFuturesTrades>>;
}

/// [`GateioFuturesBtc`] server base url.
///
/// See docs: <https://www.gate.io/docs/developers/futures/ws/en/>
pub const BASE_URL_GATEIO_FUTURES_BTC: &str = "wss://fx-ws.gateio.ws/v4/ws/btc";

/// Todo:
pub type GateioFuturesBtc = Gateio<GateioServerFuturesBtc>;

/// Todo:
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize,
)]
pub struct GateioServerFuturesBtc;

impl ServerSelector for GateioServerFuturesBtc {
    const ID: ExchangeId = ExchangeId::GateioFuturesBtc;

    fn base_url() -> &'static str {
        BASE_URL_GATEIO_FUTURES_BTC
    }
}

impl StreamSelector<PublicTrades> for GateioFuturesBtc {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, GateioFuturesTrades>>;
}
