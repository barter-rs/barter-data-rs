use super::{
    CoinbaseChannel, CoinbaseSubMeta
};
use crate::{
    exchange::{ExchangeMeta, ExchangeId},
    Identifier,
    subscriber::subscription::SubscriptionIdentifier
};
use serde::{Deserialize, Serialize};

/// [`CoinbasePro`] server base url.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview>
pub const BASE_URL_COINBASE_PRO: &'static str = "wss://ws-feed.exchange.coinbase.com";

/// [`CoinbasePro`] exchange.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct CoinbasePro;

impl Identifier<ExchangeId> for CoinbasePro {
    fn id() -> ExchangeId {
        ExchangeId::CoinbasePro
    }
}

impl<CoinbaseEvent> ExchangeMeta<CoinbaseEvent> for CoinbasePro
where
    CoinbaseEvent: SubscriptionIdentifier + Identifier<CoinbaseChannel> + for<'de> Deserialize<'de>
{
    type ExchangeSub = CoinbaseSubMeta;

    fn base_url() -> &'static str {
        BASE_URL_COINBASE_PRO
    }
}