use domain::subscription::KrakenSubMeta;
use crate::{
    exchange::{ExchangeId, ExchangeMeta},
    ExchangeIdentifier, Identifier,
};
use barter_integration::model::SubscriptionId;
use serde::{Deserialize, Serialize};

pub mod domain;

/// [`Kraken`] server base url.
///
/// See docs: <https://docs.kraken.com/websockets/#overview>
pub const BASE_URL_KRAKEN: &str = "wss://ws.kraken.com/";

/// [`Kraken`] exchange.
///
/// See docs: <https://docs.kraken.com/websockets/#overview>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Kraken;

impl ExchangeIdentifier for Kraken {
    fn exchange_id() -> ExchangeId {
        ExchangeId::Kraken
    }
}

impl<KrakenEvent> ExchangeMeta<KrakenEvent> for Kraken
where
    KrakenEvent: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
{
    type ExchangeSub = KrakenSubMeta;

    fn base_url() -> &'static str {
        BASE_URL_KRAKEN
    }
}
