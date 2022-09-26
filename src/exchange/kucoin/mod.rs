//! Websocket implementation for Kucoin websocket API.
//! 
//! There are 50 connections allowed per user ID. There is a limit of 300 topics per connection.

use barter_integration::{Validator, error::SocketError, model::{InstrumentKind, SubscriptionId}, protocol::websocket::WsMessage};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{model::{SubscriptionIds, Subscription, SubKind}, exchange::get_time};

/// [`Kucoin`] specific data structures.
pub mod model;

/// `Kucoin` [`Subscriber`] & [`ExchangeTransformer`] implementor for the collection
/// of `Spot` data.
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct Kucoin {
    pub ids: SubscriptionIds,
}

impl Kucoin {
    pub const CHANNEL_TRADES: &'static str = "/market/match:";

    /// Determine the [`Kucoin`] channel metadata associated with an input Barter [`Subscription`].
    /// This returns the topic message.
    /// 
    /// Example Ok Return: Ok("/market/match:BTC-USDT")
    pub fn build_channel_metadata(sub: &Subscription) -> Result<String, SocketError> {
        let sub = sub.validate()?;
        
        // Determine Ftx market using the Instrument
        let market = match &sub.instrument.kind {
            InstrumentKind::Spot => format!("{}-{}", sub.instrument.base.to_string().to_uppercase(), sub.instrument.quote.to_string().to_uppercase()),
            InstrumentKind::FuturePerpetual => format!("{}-PERP", sub.instrument.base),
        };

        // Determine Ftx channel using the Subscription SubKind
        match &sub.kind {
            SubKind::Trade => return Ok(format!("{}{}", Self::CHANNEL_TRADES, market)),
            other => {
                return Err(SocketError::Unsupported {
                    entity: "FEFE",//Self::EXCHANGE.as_str(),
                    item: other.to_string(),
                })
            }
        };
    }

    /// Build a [`Kucoin`] compatible subscription message using the topic provided.
    pub fn build_subscription_message(topic: &str) -> WsMessage {
        WsMessage::Text(
            json!({
                "id": get_time(),
                "type": "subscribe",
                "topic": topic,
                "privateChannel": false,
                // TODO: Deal with this response message
                "response": true,
            })
            .to_string()
        )
    }

    /// Build a [`Kucoin`] compatible [`SubscriptionId`] using the topic.
    /// Used to associate [`Kucoin`] data structures receive over the Websocket with
    /// the original Barter [`Subscription`].
    /// 
    /// ex/ SubscriptionId("/market/match:BTC-USDT")
    pub fn subscription_id(topic: &str) -> SubscriptionId {
        SubscriptionId::from(topic)
    }
}