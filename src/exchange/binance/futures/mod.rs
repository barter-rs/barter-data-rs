use super::domain::subscription::BinanceSubMeta;
use crate::{
    exchange::{ExchangeId, ExchangeMeta},
    ExchangeIdentifier, Identifier,
};
use barter_integration::model::SubscriptionId;
use serde::Deserialize;

/// Todo:
pub mod domain;

/// [`BinanceFuturesUsd`] server base url.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#websocket-market-streams>
const BASE_URL_BINANCE_FUTURES_USD: &str = "wss://fstream.binance.com/ws";

/// [`BinanceFuturesUsd`] exchange.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#websocket-market-streams>
#[derive(Debug, Clone, Copy)]
pub struct BinanceFuturesUsd;

impl ExchangeIdentifier for BinanceFuturesUsd {
    fn exchange_id() -> ExchangeId {
        ExchangeId::BinanceFuturesUsd
    }
}

impl<BinanceEvent> ExchangeMeta<BinanceEvent> for BinanceFuturesUsd
where
    BinanceEvent: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
{
    type ExchangeSub = BinanceSubMeta;

    fn base_url() -> &'static str {
        BASE_URL_BINANCE_FUTURES_USD
    }
}
