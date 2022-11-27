use super::{
    SubscriptionIdentifier,
    subscription::{
        Subscription, SubscriptionMap, SubscriptionMeta, SubKind, DomainSubscription
    }
};
use std::collections::HashMap;
use serde::de::DeserializeOwned;

pub trait SubscriptionMapper {
    fn map<Kind, ExchangeSub, ExchangeEvent>(subscriptions: &[Subscription<Kind>]) -> SubscriptionMeta<Kind>
    where
        Kind: SubKind,
        ExchangeSub: DomainSubscription<ExchangeEvent>,
        ExchangeEvent: SubscriptionIdentifier + DeserializeOwned;
}

pub struct WebSocketSubMapper;

impl SubscriptionMapper for WebSocketSubMapper {
    fn map<Kind, ExchangeSub, ExchangeEvent>(subscriptions: &[Subscription<Kind>]) -> SubscriptionMeta<Kind>
    where
        Kind: SubKind,
        ExchangeSub: DomainSubscription<ExchangeEvent>,
        ExchangeEvent: SubscriptionIdentifier + DeserializeOwned,
    {
        // Allocate SubscriptionIds HashMap to track identifiers for each actioned Subscription
        let mut map = SubscriptionMap(HashMap::with_capacity(subscriptions.len()));

        // Map Barter Subscriptions to exchange specific SubscriptionMeta
        let exchange_subs = subscriptions
            .iter()
            .map(|subscription| {
                // Translate Barter Subscription to exchange specific subscription
                let exchange_sub = ExchangeSub::new(subscription);

                // Determine the SubscriptionId associated with this exchange specific subscription
                let subscription_id = exchange_sub.subscription_id();

                // Use ExchangeSub SubscriptionId as the link to this Barter Subscription
                map.0.insert(subscription_id, subscription.clone());

                exchange_sub
            })
            .collect::<Vec<ExchangeSub>>();

        // Construct WebSocket message subscriptions requests
        let subscriptions = ExchangeSub::requests(exchange_subs);

        SubscriptionMeta {
            map,
            expected_responses: ExchangeSub::expected_responses(&subscriptions),
            subscriptions,
        }
    }
}