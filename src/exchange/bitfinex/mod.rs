//! Websocket implementation for Bitfinex v2 websocket API.
//!
//! The user is allowed up to 20 connections per minute on the public API. Each connection
//! can be used to connect up to 25 different channels.

use std::collections::HashMap;

use barter_integration::{
    error::SocketError,
    model::{InstrumentKind, SubscriptionId},
    protocol::websocket::WsMessage,
    Transformer, Validator,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    model::{MarketEvent, Subscription, SubscriptionIds, SubscriptionMeta},
    ExchangeId, ExchangeTransformer, Subscriber,
};

use self::model::{BitfinexMessage, BitfinexSubResponse};

/// [`Bitfinex`] specific data structures.
pub mod model;

/// `Bitfinex` [`Subscriber`] & [`ExchangeTransformer`] implementor for the collection
/// of `Spot` data.
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct Bitfinex {
    pub ids: SubscriptionIds,
}

impl Bitfinex {
    pub const CHANNEL_TRADES: &'static str = "trades";

    fn subscription_id(channel: &str, market: &str) -> SubscriptionId {
        SubscriptionId::from(format!("{channel}|{market}"))
    }

    pub fn build_channel_meta(sub: &Subscription) -> Result<(&str, String), SocketError> {
        let sub = sub.validate()?;

        let channel = match &sub.kind {
            crate::model::SubKind::Trade => Self::CHANNEL_TRADES,
            crate::model::SubKind::Candle(_) => todo!(),
            crate::model::SubKind::OrderBookL2 => todo!(),
        };

        // Determine the symbol name
        let market = match &sub.instrument.kind {
            InstrumentKind::Spot => format!(
                "t{}{}",
                sub.instrument.base.to_string().to_uppercase(),
                sub.instrument.quote.to_string().to_uppercase()
            ),
            InstrumentKind::FuturePerpetual => todo!(),
        };

        Ok((channel, market))
    }

    fn build_subscription_message(channel: &str, market: &str) -> WsMessage {
        WsMessage::Text(
            json!({
                "event": "subscribe",
                "channel": channel,
                "symbol": market,
            })
            .to_string(),
        )
    }
}

impl Subscriber for Bitfinex {
    type SubResponse = BitfinexSubResponse;

    fn base_url() -> &'static str {
        "wss://api-pub.bitfinex.com/ws/2"
    }

    fn build_subscription_meta(
        subscriptions: &[crate::model::Subscription],
    ) -> Result<crate::model::SubscriptionMeta, barter_integration::error::SocketError> {
        // Allocate SubscriptionIds HashMap to track identifiers for each actioned Subscription
        let mut ids = SubscriptionIds(HashMap::with_capacity(subscriptions.len()));

        // Map Barter subscriptions to Bitfinex subscriptions
        let subscriptions = subscriptions
            .iter()
            .map(|subscription| {
                // Determine the Bitfinex specific channel & market for this subscription
                let (channel, market) = Self::build_channel_meta(subscription)?;

                // Use "channel|market" as the Subscription key in the SubscriptionIds
                // eg/ SubscriptionId("trades|BTC/USDT")
                ids.insert(
                    Bitfinex::subscription_id(channel, &market),
                    subscription.clone(),
                );

                // Construct Bitfinex specific subscription message
                Ok(Self::build_subscription_message(channel, &market))
            })
            .collect::<Result<Vec<_>, SocketError>>()?;

        Ok(SubscriptionMeta {
            ids,
            expected_responses: subscriptions.len(),
            subscriptions,
        })
    }
}

impl ExchangeTransformer for Bitfinex {
    const EXCHANGE: ExchangeId = ExchangeId::Bitfinex;

    fn new(
        _: tokio::sync::mpsc::UnboundedSender<WsMessage>,
        ids: SubscriptionIds,
    ) -> Self {
        Self { ids }
    }
}

impl Transformer<MarketEvent> for Bitfinex {
    type Input = BitfinexMessage;
    type OutputIter = Vec<Result<MarketEvent, SocketError>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        println!("{:?}", input);
        //println!("{:?}", self.ids);
        match input {
            BitfinexMessage::HeartbeatEvent { .. } => {
                vec![]
            }
            // TODO: Should we ignore trade snapshot?
            BitfinexMessage::TradeSnapshotEvent { .. } => {
                vec![]
            }
            BitfinexMessage::TradeUpdateEvent {
                subscription_id,
                update_type,
                trade,
            } => {
                // TODO: Maybe not good to allocate a string each time?
                if update_type == String::from("te") {
                    let instrument = match self.ids.find_instrument(&subscription_id) {
                        Ok(instrument) => instrument,
                        Err(err) => return vec![Err(err)],
                    };

                    vec![Ok(MarketEvent::from((
                        Bitfinex::EXCHANGE,
                        instrument.clone(),
                        trade,
                    )))]
                } else {
                    vec![]
                }
            },
            // BitfinexMessage::SubscriptionEvent { subscription_id, sub_response } => {
            //     println!("IN TRADE SUBSCRIPTION MESSAGE");
            //     match sub_response {
            //         // Replace SubscriptionId to that of the channel
            //         BitfinexSubResponse::TradeSubscriptionMessage { 
            //             event, 
            //             channel, 
            //             channel_id, 
            //             symbol, 
            //             pair } => {
            //                 // Remove the generic subscription id and replace it
            //                 println!("IN TRADE SUBSCRIPTION MESSAGE");
            //                 let sub_id = Bitfinex::subscription_id(&channel, &pair);
            //                 println!("sub id: {:?}", sub_id);
            //                 if let Some(subscription) = self.ids.remove(&sub_id) {
            //                     self.ids.insert(SubscriptionId(channel_id.to_string()), subscription);
            //                 }
            //                 vec![]
            //             },
            //         // TODO: Deal with this appropriately
            //         BitfinexSubResponse::Error { .. } => todo!(),
            //     }
            // }
            _ => { vec![]},
        }
    }
}
