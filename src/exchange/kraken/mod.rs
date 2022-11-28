use self::domain::{KrakenChannel, KrakenSubMeta};
use crate::{
    exchange::{ExchangeMeta, ExchangeId},
    Identifier,
    subscriber::subscription::SubscriptionIdentifier
};
use serde::{Deserialize, Serialize};

pub mod domain;


/// [`Kraken`] server base url.
///
/// See docs: <https://docs.kraken.com/websockets/#overview>
pub const BASE_URL_KRAKEN: &'static str = "wss://ws.kraken.com/";

/// [`Kraken`] exchange.
///
/// See docs: <https://docs.kraken.com/websockets/#overview>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Kraken;

impl Identifier<ExchangeId> for Kraken {
    fn id() -> ExchangeId {
        ExchangeId::Kraken
    }
}

impl<KrakenEvent> ExchangeMeta<KrakenEvent> for Kraken
where
    KrakenEvent: SubscriptionIdentifier + Identifier<KrakenChannel> + for<'de> Deserialize<'de>,
{
    type ExchangeSub = KrakenSubMeta;

    fn base_url() -> &'static str {
        BASE_URL_KRAKEN
    }
}
