use super::CoinbaseSubMeta;
use crate::{
    exchange::{ExchangeId, ExchangeMeta},
    ExchangeIdentifier, Identifier,
};
use barter_integration::model::SubscriptionId;
use serde::{Deserialize, Serialize};

/// [`CoinbasePro`] server base url.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview>
pub const BASE_URL_COINBASE_PRO: &str = "wss://ws-feed.exchange.coinbase.com";

/// [`CoinbasePro`] exchange.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct CoinbasePro;

impl ExchangeIdentifier for CoinbasePro {
    fn exchange_id() -> ExchangeId {
        ExchangeId::CoinbasePro
    }
}

impl<CoinbaseEvent> ExchangeMeta<CoinbaseEvent> for CoinbasePro
where
    CoinbaseEvent: Identifier<SubscriptionId> + for<'de> Deserialize<'de>,
{
    type ExchangeSub = CoinbaseSubMeta;

    fn base_url() -> &'static str {
        BASE_URL_COINBASE_PRO
    }
}
