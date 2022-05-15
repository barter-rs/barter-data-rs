use super::{BinanceSubResponse, BinanceMessage};
use crate::{ExchangeTransformerId, Subscriber, ExchangeTransformer, Subscription, SubscriptionMeta, SubscriptionIds, model::MarketData, Identifiable};
use barter_integration::{
    StreamKind,
    socket::{
        Transformer, error::SocketError
    }
};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct BinanceFutures {
    pub ids: SubscriptionIds,
}

impl Subscriber for BinanceFutures {
    type SubResponse = BinanceSubResponse;

    fn base_url() -> &'static str { "wss://fstream.binance.com/ws" }

    fn build_subscription_meta(subscriptions: &[Subscription]) -> Result<SubscriptionMeta, SocketError> {
        // Allocate SubscriptionIds HashMap to track identifiers for each actioned Subscription
        let mut ids = SubscriptionIds(HashMap::with_capacity(subscriptions.len()));

        // Todo:

        Ok(SubscriptionMeta {
            ids,
            expected_responses: 0,
            subscriptions: vec![]
        })
    }
}

impl ExchangeTransformer for BinanceFutures {
    const EXCHANGE: ExchangeTransformerId = ExchangeTransformerId::BinanceFutures;
    fn new(ids: SubscriptionIds) -> Self { Self { ids } }
}

impl Transformer<MarketData> for BinanceFutures {
    type Input = BinanceMessage;
    type OutputIter = Vec<Result<MarketData, SocketError>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        let market_data = self
            .ids
            .find_instrument(input.id())
            .map(|instrument| {
                MarketData::from((BinanceFutures::EXCHANGE, instrument, input))
            });

        vec![market_data]
    }
}

impl BinanceFutures {
    fn get_channel_id(sub: &Subscription) -> Result<String, SocketError> {
        match &sub.kind {
            StreamKind::Trades => Ok(format!("{}{}@aggTrade", sub.instrument.base, sub.instrument.quote)),
            other =>  Err(SocketError::Unsupported {
                entity: BinanceFutures::EXCHANGE.as_str(),
                item: other.to_string()
            })
        }
    }
}