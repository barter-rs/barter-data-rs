use super::{BinanceMessage, BinanceSubResponse};
use crate::{
    model::MarketData, ExchangeTransformer, ExchangeTransformerId, Identifiable, Subscriber,
    Subscription, SubscriptionIds, SubscriptionMeta,
};
use barter_integration::socket::protocol::websocket::WsMessage;
use barter_integration::{
    socket::{error::SocketError, Transformer},
    StreamKind, SubscriptionId,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct BinanceFutures {
    pub ids: SubscriptionIds,
}

impl Subscriber for BinanceFutures {
    type SubResponse = BinanceSubResponse;

    fn base_url() -> &'static str {
        "wss://fstream.binance.com/ws"
    }

    fn build_subscription_meta(
        subscriptions: &[Subscription],
    ) -> Result<SubscriptionMeta, SocketError> {
        // Allocate SubscriptionIds HashMap to track identifiers for each actioned Subscription
        let mut ids = SubscriptionIds(HashMap::with_capacity(subscriptions.len()));

        // Map Barter Subscriptions to BinanceFutures channels
        let channels = subscriptions
            .iter()
            .map(|subscription| {
                // Determine the BinanceFutures specific channel for this Barter Subscription
                let channel = Self::get_channel_id(subscription)?;

                // Use channel as the SubscriptionId key in the SubscriptionIds
                ids.0
                    .insert(SubscriptionId(channel.clone()), subscription.clone());

                Ok(channel)
            })
            .collect::<Result<Vec<_>, SocketError>>()?;

        // Use channels to construct a Binance subscription WsMessage
        let subscriptions = Self::subscriptions(channels);

        Ok(SubscriptionMeta {
            ids,
            expected_responses: subscriptions.len(),
            subscriptions,
        })
    }
}

impl ExchangeTransformer for BinanceFutures {
    const EXCHANGE: ExchangeTransformerId = ExchangeTransformerId::BinanceFutures;
    fn new(ids: SubscriptionIds) -> Self {
        Self { ids }
    }
}

impl Transformer<MarketData> for BinanceFutures {
    type Input = BinanceMessage;
    type OutputIter = Vec<Result<MarketData, SocketError>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        let market_data = self
            .ids
            .find_instrument(input.id())
            .map(|instrument| MarketData::from((BinanceFutures::EXCHANGE, instrument, input)));

        vec![market_data]
    }
}

impl BinanceFutures {
    /// Determine the Binance channel identifier associated with an input Barter [`Subscription`].
    fn get_channel_id(sub: &Subscription) -> Result<String, SocketError> {
        match &sub.kind {
            StreamKind::Trade => Ok(format!(
                "{}{}@aggTrade",
                sub.instrument.base, sub.instrument.quote
            )),
            other => Err(SocketError::Unsupported {
                entity: BinanceFutures::EXCHANGE.as_str(),
                item: other.to_string(),
            }),
        }
    }

    /// Build a `BinanceFutures` compatible subscription message using the channels provided.
    fn subscriptions(channels: Vec<String>) -> Vec<WsMessage> {
        vec![WsMessage::Text(
            json!({
                "method": "SUBSCRIBE",
                "params": channels,
                "id": 1
            })
            .to_string(),
        )]
    }
}
