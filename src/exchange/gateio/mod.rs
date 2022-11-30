use self::domain::{GateioMessage, GateioSubResult};
use crate::{
    Identifier,
    subscriber::subscription::{ExchangeSubscription, SubKind, Subscription, trade::PublicTrades},
};
use barter_integration::{
    model::{InstrumentKind, SubscriptionId},
    protocol::websocket::WsMessage,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use chrono::Utc;

/// Todo:
pub mod domain;
pub mod future;
pub mod spot;

/// Todo:
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct GateioChannel(&'static str);

impl GateioChannel {
    /// Gateio [`InstrumentKind::Spot`] real-time trades channel.
    ///
    /// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#public-trades-channel>
    const SPOT_TRADES: Self = Self("spot.trades");


    /// Gateio [`InstrumentKind::FuturePerpetual`] real-time trades channel.
    ///
    /// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#public-trades-channel>
    const FUTURE_PERPETUAL_TRADES: Self = Self("futures.trades");
}

impl Identifier<GateioChannel> for Subscription<PublicTrades> {
    fn id(&self) -> GateioChannel {
        match self.instrument.kind {
            InstrumentKind::Spot => GateioChannel::SPOT_TRADES,
            InstrumentKind::FuturePerpetual => GateioChannel::FUTURE_PERPETUAL_TRADES,
        }
    }
}

/// Todo:
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct GateioSubMeta {
    channel: GateioChannel,
    market: String,
}

impl Identifier<SubscriptionId> for GateioSubMeta {
    fn id(&self) -> SubscriptionId {
        subscription_id(self.channel.0, &self.market)
    }
}

impl<ExchangeEvent> ExchangeSubscription<ExchangeEvent> for GateioSubMeta
where
    ExchangeEvent: Identifier<SubscriptionId> + for<'de> Deserialize<'de>,
{
    type Channel = GateioChannel;
    type SubResponse = GateioMessage<GateioSubResult>;

    fn new<Kind>(sub: &Subscription<Kind>) -> Self
    where
        Kind: SubKind,
        Subscription<Kind>: Identifier<Self::Channel>,
    {
        Self {
            channel: sub.id(),
            market: format!("{}_{}", sub.instrument.base, sub.instrument.quote).to_uppercase()
        }
    }

    fn requests(subscriptions: Vec<Self>) -> Vec<WsMessage> {
        subscriptions
            .into_iter()
            .map(|Self { channel, market }| {
                WsMessage::Text(
                    json!({
                        "time": Utc::now().timestamp_millis(),
                        "channel": channel.0,
                        "event": "subscribe",
                        "payload": [market]
                    })
                    .to_string()
                )
            })
            .collect()
    }
}

/// Generate an Gateio [`SubscriptionId`] from the channel and market provided.
///
/// Uses "instrument_kind.channel|MARKET":
/// eg/ SubscriptionId("spot.trades|ETH_USD")
pub(crate) fn subscription_id(channel: &str, market: &str) -> SubscriptionId {
    SubscriptionId::from(format!("{}|{}", channel, market))
}

