use crate::{
    subscriber::subscription::{ExchangeSubscription, SubKind, Subscription, trade::PublicTrades},
    Identifier,
};
use barter_integration::{model::{SubscriptionId, InstrumentKind}, protocol::websocket::WsMessage, Validator};
use serde::{Deserialize, Serialize};
use barter_integration::error::SocketError;

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
        subscription_id(self.channel, &self.market)
    }
}

impl<ExchangeEvent> ExchangeSubscription<ExchangeEvent> for GateioSubMeta
where
    ExchangeEvent: Identifier<SubscriptionId> + for<'de> Deserialize<'de>,
{
    type Channel = GateioChannel;
    type SubResponse = GateioSubResponse;

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
        // vec![WsMessage::Text(
        //     json!({
        //         "time": Utc::now().timestamp_millis(),
        //         "channel": CHANNEL_SPOT_TRADE,
        //         "event": "subscribe",
        //         "payload": channels
        //     })
        //         .to_string()
        // )]
        todo!()
    }
}

/// Generate an Gateio [`SubscriptionId`] from the channel and market provided.
///
/// Uses "instrument_kind.channel|MARKET":
/// eg/ SubscriptionId("spot.trades|ETH_USD")
pub(crate) fn subscription_id(channel: GateioChannel, market: &str) -> SubscriptionId {
    SubscriptionId::from(format!("{}|{}", channel.0, market))
}

/// Todo:
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct GateioSubResponse;

impl Validator for GateioSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized
    {
        todo!()
    }
}