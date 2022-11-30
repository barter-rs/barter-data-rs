use crate::exchange::gateio::GateioSubMeta;
use crate::{
    exchange::{ExchangeMeta, ExchangeId},
    ExchangeIdentifier, Identifier,
};
use barter_integration::model::SubscriptionId;
use serde::Deserialize;

/// Todo:
pub mod domain;

/// [`GateioSpot`] server base url.
///
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/>
pub const BASE_URL_GATEIO_SPOT: &'static str = "wss://api.gateio.ws/ws/v4/";

/// [`GateioSpot`] exchange.
///
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/>
#[derive(Debug, Clone, Copy)]
pub struct GateioSpot;

impl ExchangeIdentifier for GateioSpot {
    fn exchange_id() -> ExchangeId {
        ExchangeId::BinanceSpot
    }
}

impl<GateioEvent> ExchangeMeta<GateioEvent> for GateioSpot
where
    GateioEvent: Identifier<SubscriptionId> + for<'de> Deserialize<'de>,
{
    type ExchangeSub = GateioSubMeta;

    fn base_url() -> &'static str {
        BASE_URL_GATEIO_SPOT
    }
}
