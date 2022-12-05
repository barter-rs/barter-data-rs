use self::domain::subscription::CoinbaseSubMeta;
use crate::{
    exchange::{ExchangeId, ExchangeMeta},
    ExchangeIdentifier, Identifier,
};
use barter_integration::model::SubscriptionId;
use serde::{Deserialize, Serialize};

/// Todo:
pub mod domain;

/// [`Coinbase`] server base url.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview>
pub const BASE_URL_COINBASE_PRO: &str = "wss://ws-feed.exchange.coinbase.com";

/// [`Coinbase`] exchange.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Coinbase;

impl ExchangeIdentifier for Coinbase {
    fn exchange_id() -> ExchangeId {
        ExchangeId::Coinbase
    }
}

impl<CoinbaseEvent> ExchangeMeta<CoinbaseEvent> for Coinbase
where
    CoinbaseEvent: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
{
    type ExchangeSub = CoinbaseSubMeta;

    fn base_url() -> &'static str {
        BASE_URL_COINBASE_PRO
    }
}
