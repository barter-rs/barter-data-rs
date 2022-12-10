use self::domain::subscription::BitfinexSubMeta;
use crate::{
    exchange::{ExchangeId, ExchangeMeta},
    ExchangeIdentifier, Identifier,
};
use barter_integration::model::SubscriptionId;
use serde::{Deserialize, Serialize};

/// Todo:
pub mod domain;
pub mod validator;

/// [`Bitfinex`] server base url.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
pub const BASE_URL_BITFINEX: &str = "wss://api-pub.bitfinex.com/ws/2";

/// [`Bitfinex`] exchange.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Bitfinex;

impl ExchangeIdentifier for Bitfinex {
    fn exchange_id() -> ExchangeId {
        ExchangeId::Bitfinex
    }
}

impl<CoinbaseEvent> ExchangeMeta<CoinbaseEvent> for Bitfinex
where
    CoinbaseEvent: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
{
    type ExchangeSub = BitfinexSubMeta;

    fn base_url() -> &'static str {
        BASE_URL_BITFINEX
    }
}