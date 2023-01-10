use self::trade::GateioFuturesTrades;
use super::{Gateio, GateioServer};
use crate::{
    exchange::{Connector, ExchangeId},
    subscription::trade::PublicTrades,
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream, StreamSelector,
};
use barter_macro::{DeExchange, SerExchange};
use serde::{Deserialize, Serialize};

/// Todo:
pub mod trade;

/// [`GateioFuturesUsd`] WebSocket server base url.
///
/// See docs: <https://www.gate.io/docs/developers/futures/ws/en/>
pub const WEBSOCKET_BASE_URL_GATEIO_FUTURES_USD: &str = "wss://fx-ws.gateio.ws/v4/ws/usdt";

/// Todo:
pub type GateioFuturesUsd = Gateio<GateioServerFuturesUsd>;

/// Todo:
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, DeExchange, SerExchange,
)]
pub struct GateioServerFuturesUsd;

impl GateioServer for GateioServerFuturesUsd {
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

/// Todo:
pub type GateioFuturesBtc = Gateio<GateioServerFuturesBtc>;

/// Todo:
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize,
)]
pub struct GateioServerFuturesBtc;

impl GateioServer for GateioServerFuturesBtc {
    const ID: ExchangeId = ExchangeId::GateioFuturesBtc;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_GATEIO_FUTURES_BTC
    }
}

impl StreamSelector<PublicTrades> for GateioFuturesBtc {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, GateioFuturesTrades>>;
}

impl<'de> serde::Deserialize<'de> for GateioFuturesUsd {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        match <String as serde::Deserialize>::deserialize(deserializer)?.as_str() {
            "GateioFuturesUsd" | "gateio_futures_usd" => Ok(Self::default()),
            other => Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(other),
                &"GateioFuturesUsd | gateio_futures_usd",
            )),
        }
    }
}

impl serde::Serialize for GateioFuturesUsd {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(GateioFuturesUsd::ID.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for GateioFuturesBtc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        match <String as serde::Deserialize>::deserialize(deserializer)?.as_str() {
            "GateioFuturesBtc" | "gateio_futures_btc" => Ok(Self::default()),
            other => Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(other),
                &"GateioFuturesBtc | gateio_futures_btc",
            )),
        }
    }
}

impl serde::Serialize for GateioFuturesBtc {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(GateioFuturesBtc::ID.as_str())
    }
}
