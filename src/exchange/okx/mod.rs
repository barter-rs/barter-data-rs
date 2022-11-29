use self::domain::{OkxChannel, OkxSubMeta};
use crate::{
    exchange::{ExchangeMeta, ExchangeId},
    Identifier,
    subscriber::subscription::SubscriptionIdentifier
};
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

impl Identifier<ExchangeId> for Okx {
    fn id() -> ExchangeId {
        ExchangeId::Okx
    }
}

impl<OkxEvent> ExchangeMeta<OkxEvent> for Okx
where
    OkxEvent: SubscriptionIdentifier + Identifier<OkxChannel> + for<'de> Deserialize<'de>
{
    type ExchangeSub = OkxSubMeta;

    fn base_url() -> &'static str {
        BASE_URL_OKX
    }
}