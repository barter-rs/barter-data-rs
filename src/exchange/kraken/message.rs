use super::trade::KrakenTrades;
use crate::{
    event::{MarketIter, PublicTrade},
    exchange::ExchangeId,
    Identifier,
};
use barter_integration::model::{Instrument, SubscriptionId};
use serde::{Deserialize, Serialize};

/// [`Kraken`] message variants that can be received over [`WebSocket`](crate::WebSocket).
///
/// See docs: <https://docs.kraken.com/websockets/#overview>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum KrakenMessage {
    Trades(KrakenTrades),
    Event(KrakenEvent),
}

impl Identifier<Option<SubscriptionId>> for KrakenMessage {
    fn id(&self) -> Option<SubscriptionId> {
        match self {
            KrakenMessage::Trades(trades) => Some(trades.subscription_id.clone()),
            KrakenMessage::Event(_) => None,
        }
    }
}

impl From<(ExchangeId, Instrument, KrakenMessage)> for MarketIter<PublicTrade> {
    fn from((exchange_id, instrument, message): (ExchangeId, Instrument, KrakenMessage)) -> Self {
        match message {
            KrakenMessage::Trades(trades) => Self::from((exchange_id, instrument, trades)),
            KrakenMessage::Event(_) => Self(vec![]),
        }
    }
}

/// [`Kraken`] messages received over the WebSocket which are not subscription data.
///
/// eg/ [`Kraken`] sends a [`KrakenEvent::Heartbeat`] if no subscription traffic has been sent
/// within the last second.
///
/// ## Examples
/// ### Heartbeat
/// ```json
/// {
///   "event": "heartbeat"
/// }
/// ```
///
/// See docs: <https://docs.kraken.com/websockets/#message-heartbeat>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum KrakenEvent {
    Heartbeat,
    Error(KrakenError),
}

/// [`Kraken`] generic error message String received over the WebSocket.
///
/// Note that since the [`KrakenError`] is only made up of a renamed message String field, it can
/// be used flexible as a [`KrakenSubResponse::Error`](KrakenSubResponse) or as a generic error
/// received over the WebSocket while subscriptions are active.
///
/// ## Examples
/// ### Subscription Error Response
/// ```json
/// {
///   "errorMessage": "Subscription depth not supported"
/// }
/// ```
///
/// See docs generic: <https://docs.kraken.com/websockets/#errortypes>
/// See docs subscription failed: <https://docs.kraken.com/websockets/#message-subscriptionStatus>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct KrakenError {
    #[serde(alias = "errorMessage")]
    pub message: String,
}
