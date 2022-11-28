use super::{
    BinanceChannel, BinanceSubMeta,
};
use crate::{
    subscriber::{
        subscription::{SubscriptionIdentifier, ExchangeMeta},
    },
    exchange::ExchangeId,
    Identifier
};
use serde::Deserialize;

/// [`BinanceSpot`] server base url.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#websocket-market-streams>
pub const BASE_URL_BINANCE_SPOT: &'static str = "wss://stream.binance.com:9443/ws";

/// [`BinanceSpot`] exchange.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#websocket-market-streams>
#[derive(Debug, Clone, Copy)]
pub struct BinanceSpot;

impl Identifier<ExchangeId> for BinanceSpot {
    fn id() -> ExchangeId {
        ExchangeId::BinanceSpot
    }
}

impl<BinanceEvent> ExchangeMeta<BinanceEvent> for BinanceSpot
where
    BinanceEvent: SubscriptionIdentifier + Identifier<BinanceChannel> + for<'de> Deserialize<'de>,
{
    type ExchangeSub = BinanceSubMeta;

    fn base_url() -> &'static str {
        BASE_URL_BINANCE_SPOT
    }
}