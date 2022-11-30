use super::super::GateioSubMeta;
use crate::{
    exchange::{ExchangeId, ExchangeMeta},
    ExchangeIdentifier, Identifier,
};
use barter_integration::model::SubscriptionId;
use serde::Deserialize;

/// [`GateioFuturesUsd`] server base url.
///
/// See docs: <https://www.gate.io/docs/developers/futures/ws/en/>
pub const BASE_URL_GATEIO_FUTURES_USD: &'static str = "wss://fx-ws.gateio.ws/v4/ws/usdt";

/// [`GateioFuturesBtc`] server base url.
///
/// See docs: <https://www.gate.io/docs/developers/futures/ws/en/>
pub const BASE_URL_GATEIO_FUTURES_BTC: &'static str = "wss://fx-ws.gateio.ws/v4/ws/btc";

/// [`GateioFuturesUsd`] exchange.
///
/// See docs: <https://www.gate.io/docs/developers/futures/ws/en/>
#[derive(Debug, Clone, Copy)]
pub struct GateioFuturesUsd;

impl ExchangeIdentifier for GateioFuturesUsd {
    fn exchange_id() -> ExchangeId {
        ExchangeId::GateioFuturesUsd
    }
}

impl<GateioEvent> ExchangeMeta<GateioEvent> for GateioFuturesUsd
where
    GateioEvent: Identifier<SubscriptionId> + for<'de> Deserialize<'de>,
{
    type ExchangeSub = GateioSubMeta;

    fn base_url() -> &'static str {
        BASE_URL_GATEIO_FUTURES_USD
    }
}

/// [`GateioFuturesBtc`] exchange.
///
/// See docs: <https://www.gate.io/docs/developers/futures/ws/en/>
#[derive(Debug, Clone, Copy)]
pub struct GateioFuturesBtc;

impl ExchangeIdentifier for GateioFuturesBtc {
    fn exchange_id() -> ExchangeId {
        ExchangeId::GateioFuturesBtc
    }
}

impl<GateioEvent> ExchangeMeta<GateioEvent> for GateioFuturesBtc
where
    GateioEvent: Identifier<SubscriptionId> + for<'de> Deserialize<'de>,
{
    type ExchangeSub = GateioSubMeta;

    fn base_url() -> &'static str {
        BASE_URL_GATEIO_FUTURES_BTC
    }
}