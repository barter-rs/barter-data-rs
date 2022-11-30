use self::domain::OkxSubMeta;
use crate::{
    exchange::{ExchangeMeta, ExchangeId},
    ExchangeIdentifier, Identifier,
};
use barter_integration::model::SubscriptionId;
use serde::{Deserialize, Serialize};

/// Todo:
pub mod domain;

/// [`Okx`] server base url.
///
/// See docs: <https://www.okx.com/docs-v5/en/#overview-api-resources-and-support>
pub const BASE_URL_OKX: &'static str = "wss://wsaws.okx.com:8443/ws/v5/public";

/// Todo:
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Okx;

impl ExchangeIdentifier for Okx {
    fn exchange_id() -> ExchangeId {
        ExchangeId::Okx
    }
}

impl<OkxEvent> ExchangeMeta<OkxEvent> for Okx
where
    OkxEvent: Identifier<SubscriptionId> + for<'de> Deserialize<'de>
{
    type ExchangeSub = OkxSubMeta;

    fn base_url() -> &'static str {
        BASE_URL_OKX
    }
}