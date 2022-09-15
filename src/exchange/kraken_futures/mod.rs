use crate::{
    ExchangeId, ExchangeTransformer, MarketEvent, Subscriber, Subscription, SubscriptionIds,
    SubscriptionMeta,
};
use barter_integration::{
    error::SocketError, model::SubscriptionId, protocol::websocket::WsMessage, Transformer,
};
use model::{
    KrakenFuturesUsdMessage, KrakenFuturesUsdSubKind, KrakenFuturesUsdSubResponse,
    KrakenFuturesUsdSubscription,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

/// [`KrakenFuturesUsd`] specific data structures.
pub mod model;

/// [`KrakenFuturesUsd`] [`Subscriber`] & [`ExchangeTransformer`] implementor for
/// the collection of `Spot` data.
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct KrakenFuturesUsd {
    pub ids: SubscriptionIds,
}

impl Subscriber for KrakenFuturesUsd {
    type SubResponse = KrakenFuturesUsdSubResponse;

    fn base_url() -> &'static str {
        "wss://futures.kraken.com/ws/v1"
    }

    fn build_subscription_meta(
        subscriptions: &[Subscription],
    ) -> Result<SubscriptionMeta, SocketError> {
        // Allocate SubscriptionIds HashMap to track identifiers for each actioned Subscription
        let mut ids = SubscriptionIds(HashMap::with_capacity(subscriptions.len()));

        // Map Barter Subscriptions to Kraken Subscriptions
        let subscriptions = subscriptions
            .iter()
            .map(|subscription| {
                // Translate Barter Subscription to the associated KrakenSubscription
                let kraken_subscription = KrakenFuturesUsd::subscription(subscription)?;

                // Determine the SubscriptionId ("{channel}|{market} ")for this KrakenSubscription
                // eg/ SubscriptionId("ohlc-5|XBT/USD")
                let subscription_id = SubscriptionId::from(&kraken_subscription);

                // Insert SubscriptionId to Barter Subscription Entry in SubscriptionIds HashMap
                ids.insert(subscription_id, subscription.clone());

                WsMessage::try_from(&kraken_subscription)
            })
            .collect::<Result<Vec<_>, SocketError>>()?;

        Ok(SubscriptionMeta {
            ids,
            expected_responses: subscriptions.len(),
            subscriptions,
        })
    }
}

impl ExchangeTransformer for KrakenFuturesUsd {
    const EXCHANGE: ExchangeId = ExchangeId::KrakenFuturesUsd;

    fn new(_: mpsc::UnboundedSender<WsMessage>, ids: SubscriptionIds) -> Self {
        Self { ids }
    }
}

impl Transformer<MarketEvent> for KrakenFuturesUsd {
    type Input = KrakenFuturesUsdMessage;
    type OutputIter = Vec<Result<MarketEvent, SocketError>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        match input {
            KrakenFuturesUsdMessage::Trade(trade) => {
                let instrument = match self.ids.find_instrument(trade.product_id.clone()) {
                    Ok(instrument) => instrument,
                    Err(error) => {
                        return vec![Err(error)];
                    }
                };
                vec![Ok(MarketEvent::from((
                    KrakenFuturesUsd::EXCHANGE,
                    instrument.clone(),
                    trade,
                )))]
            }
            // TradeSnapshot consists of old trades, so are ignored
            KrakenFuturesUsdMessage::TradeSnapshot { .. } => {
                vec![]
                /*
                let instrument = match self.ids.find_instrument(product_id.clone()) {
                    Ok(instrument) => instrument,
                    Err(error) => {
                        return vec![Err(error)];
                    }
                };

                trades
                    .into_iter()
                    .map(|trade| {
                        Ok(MarketEvent::from((
                            KrakenFuturesUsd::EXCHANGE,
                            instrument.clone(),
                            trade,
                        )))
                    })
                    .collect()
                */
            }
        }
    }
}

impl KrakenFuturesUsd {
    /// Translate a Barter [`Subscription`] into a [`KrakenFuturesUsd`] compatible subscription message.
    fn subscription(sub: &Subscription) -> Result<KrakenFuturesUsdSubscription, SocketError> {
        // Determine Kraken pair using the Instrument
        let pair = format!("PI_{}{}", sub.instrument.base, sub.instrument.quote).to_uppercase();

        // Determine the KrakenSubKind from the Barter SubKind
        let kind = KrakenFuturesUsdSubKind::try_from(&sub.kind)?;

        Ok(KrakenFuturesUsdSubscription::new(pair, kind))
    }
}
