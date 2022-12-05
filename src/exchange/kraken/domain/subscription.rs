use super::KrakenError;
use crate::{
    subscriber::subscription::{trade::PublicTrades, ExchangeSubscription, SubKind, Subscription},
    Identifier,
};
use barter_integration::{
    error::SocketError, model::SubscriptionId, protocol::websocket::WsMessage, Validator,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Todo:
///
/// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct KrakenChannel(&'static str);

impl KrakenChannel {
    /// [`Kraken`] real-time trades channel name.
    ///
    /// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
    const TRADES: Self = Self("trade");
}

impl Identifier<KrakenChannel> for Subscription<PublicTrades> {
    fn id(&self) -> KrakenChannel {
        KrakenChannel::TRADES
    }
}

/// Todo:
///
/// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct KrakenSubMeta {
    channel: KrakenChannel,
    market: String,
}

impl Identifier<SubscriptionId> for KrakenSubMeta {
    fn id(&self) -> SubscriptionId {
        subscription_id(self.channel, &self.market)
    }
}

impl<ExchangeEvent> ExchangeSubscription<ExchangeEvent> for KrakenSubMeta
where
    ExchangeEvent: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
{
    type Channel = KrakenChannel;
    type SubResponse = KrakenSubResponse;

    fn new<Kind>(sub: &Subscription<Kind>) -> Self
    where
        Kind: SubKind,
        Subscription<Kind>: Identifier<Self::Channel>,
    {
        Self {
            channel: sub.id(),
            market: format!("{}/{}", sub.instrument.base, sub.instrument.quote).to_uppercase(),
        }
    }

    fn requests(subscriptions: Vec<Self>) -> Vec<WsMessage> {
        subscriptions
            .into_iter()
            .map(|Self { channel, market }| {
                WsMessage::Text(
                    json!({
                        "event": "subscribe",
                        "pair": [market],
                        "subscription": {
                            "name": channel.0
                        }
                    })
                    .to_string(),
                )
            })
            .collect()
    }
}

/// Generate a [`Kraken`] [`SubscriptionId`] from the channel and market provided.
///
/// Uses "channel|BASE/QUOTE":
/// eg/ SubscriptionId::from("trade|XBT/USD")
pub(crate) fn subscription_id(channel: KrakenChannel, market: &str) -> SubscriptionId {
    SubscriptionId::from(format!("{}|{}", channel.0, market))
}

/// [`Kraken`] message received in response to WebSocket subscription requests.
///
/// ## Examples
/// ### Subscription Trade Ok Response
/// ```json
/// {
///   "channelID": 10001,
///   "channelName": "ticker",
///   "event": "subscriptionStatus",
///   "pair": "XBT/EUR",
///   "status": "subscribed",
///   "subscription": {
///     "name": "ticker"
///   }
/// }
/// ```
///
/// ### Subscription Trade Error Response
/// ```json
/// {
///   "errorMessage": "Subscription name invalid",
///   "event": "subscriptionStatus",
///   "pair": "XBT/USD",
///   "status": "error",
///   "subscription": {
///     "name": "trades"
///   }
/// }
/// ```
///
/// See docs: <https://docs.kraken.com/websockets/#message-subscriptionStatus>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum KrakenSubResponse {
    Subscribed {
        #[serde(alias = "channelID")]
        channel_id: u64,
        #[serde(alias = "channelName")]
        channel_name: String,
        pair: String,
    },
    Error(KrakenError),
}

impl Validator for KrakenSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self {
            KrakenSubResponse::Subscribed { .. } => Ok(self),
            KrakenSubResponse::Error(error) => Err(SocketError::Subscribe(format!(
                "received failure subscription response: {}",
                error.message
            ))),
        }
    }
}
