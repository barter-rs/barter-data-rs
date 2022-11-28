use crate::{
    subscriber::subscription::{ExchangeSubscription, SubKind, Subscription, SubscriptionIdentifier},
    Identifier
};
use barter_integration::{
    error::SocketError,
    model::SubscriptionId,
    protocol::websocket::WsMessage,
    Validator
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use barter_integration::model::InstrumentKind;

/// Todo:
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Debug, Copy, Clone)]
pub struct OkxChannel(&'static str);

impl OkxChannel {
    /// [`Okx`] real-time trades channel.
    ///
    /// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel-trades-channel>
    const TRADES: Self = Self("trades");
}

/// Todo:
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
pub struct OkxSubMeta {
    channel: OkxChannel,
    #[serde(rename = "inst_id")]
    market: String,
}

impl SubscriptionIdentifier for OkxSubMeta {
    fn subscription_id(&self) -> SubscriptionId {
        subscription_id(self.channel, &self.market)
    }
}

impl<ExchangeEvent> ExchangeSubscription<ExchangeEvent> for OkxSubMeta
where
    ExchangeEvent: SubscriptionIdentifier + Identifier<OkxChannel> + for<'de> Deserialize<'de>,
{
    type SubResponse = OkxSubResponse;

    fn new<Kind>(sub: &Subscription<Kind>) -> Self
    where
        Kind: SubKind
    {
        Self {
            channel: ExchangeEvent::id(),
            // Todo: How do we indicate we want futures?
            market: format!("{}-{}", sub.instrument.base, sub.instrument.quote).to_uppercase()
        }
    }

    fn requests(subscriptions: Vec<Self>) -> Vec<WsMessage> {
        vec![WsMessage::Text(
            json!({
                "op": "subscribe",
                "args": subscriptions,
            })
            .to_string()
        )]
    }
}

/// Generate an [`Okx`] [`SubscriptionId`] from the channel and market provided.
///
/// Uses "channel|MARKET":
/// eg/ SubscriptionId("trades|ETH-USD")
pub(crate) fn subscription_id(channel: OkxChannel, market: &str) -> SubscriptionId {
    SubscriptionId::from(format!("{}|{}", channel.0, market))
}

pub struct OkxSubResponse;