use super::{
    BinanceChannel, BinanceSubMeta,
};
use crate::{
    exchange::{ExchangeMeta, ExchangeId},
    Identifier,
    subscriber::subscription::SubscriptionIdentifier
};
use serde::Deserialize;

/// Todo:
pub mod liquidation;

/// [`BinanceFuturesUsd`] server base url.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#websocket-market-streams>
const BASE_URL_BINANCE_FUTURES_USD: &'static str = "wss://fstream.binance.com/ws";

/// [`BinanceFuturesUsd`] exchange.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#websocket-market-streams>
#[derive(Debug, Clone, Copy)]
pub struct BinanceFuturesUsd;

impl Identifier<ExchangeId> for BinanceFuturesUsd {
    fn id() -> ExchangeId {
        ExchangeId::BinanceFuturesUsd
    }
}

impl<BinanceEvent> ExchangeMeta<BinanceEvent> for BinanceFuturesUsd
where
    BinanceEvent: SubscriptionIdentifier + Identifier<BinanceChannel> + for<'de> Deserialize<'de>,
{
    type ExchangeSub = BinanceSubMeta;

    fn base_url() -> &'static str { BASE_URL_BINANCE_FUTURES_USD }
}