use crate::{
    Identifier,
    subscriber::subscription::{ExchangeSubscription, SubKind, Subscription, trade::PublicTrades}
};
use barter_integration::{
    error::SocketError,
    model::SubscriptionId,
    protocol::websocket::WsMessage,
    Validator
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Todo:
pub mod trade;

/// Todo:
///
/// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct KrakenChannel(&'static str);

impl KrakenChannel {
    /// [`Kraken`] real-time trades channel name.
    ///
    /// See docs: <> Todo:
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
    ExchangeEvent: Identifier<SubscriptionId> + for<'de> Deserialize<'de>
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
                    .to_string()
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
/// eg/ KrakenSubResponse::Subscribed {
///     "channelID":337,
///     "channelName":"trade",
///     "event":"subscriptionStatus",
///     "pair":"XBT/USD",
///     "status":"subscribed",
///     "subscription":{"name":"trade"}
/// }
/// eg/ KrakenSubResponse::Error {
///     "errorMessage":"Subscription name invalid",
///     "event":"subscriptionStatus",
///     "pair":"ETH/USD",
///     "status":"error",
///     "subscription":{"name":"trades"}
/// }
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

/// [`Kraken`] generic error message String received over the WebSocket.
///
/// Note that since the [`KrakenError`] is only made up of a renamed message String field, it can
/// be used flexible as a [`KrakenSubResponse::Error`](KrakenSubResponse) or as a generic error
/// received over the WebSocket while subscriptions are active.
///
/// See docs generic: <https://docs.kraken.com/websockets/#errortypes>
/// See docs subscription failed: <https://docs.kraken.com/websockets/#message-subscriptionStatus>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct KrakenError {
    #[serde(alias = "errorMessage")]
    pub message: String,
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
