use super::BinanceSubMeta;
use crate::{
    exchange::{ExchangeId, ExchangeMeta},
    ExchangeIdentifier, Identifier,
};
use barter_integration::model::SubscriptionId;
use serde::Deserialize;

/// [`BinanceSpot`] server base url.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#websocket-market-streams>
pub const BASE_URL_BINANCE_SPOT: &str = "wss://stream.binance.com:9443/ws";

/// [`BinanceSpot`] exchange.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#websocket-market-streams>
#[derive(Debug, Clone, Copy)]
pub struct BinanceSpot;

impl ExchangeIdentifier for BinanceSpot {
    fn exchange_id() -> ExchangeId {
        ExchangeId::BinanceSpot
    }
}

impl<BinanceEvent> ExchangeMeta<BinanceEvent> for BinanceSpot
where
    BinanceEvent: Identifier<SubscriptionId> + for<'de> Deserialize<'de>,
{
    type ExchangeSub = BinanceSubMeta;

    fn base_url() -> &'static str {
        BASE_URL_BINANCE_SPOT
    }
}
