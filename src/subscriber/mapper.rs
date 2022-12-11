use std::collections::HashMap;
use crate::exchange::ExchangeSubscription;
use crate::Identifier;
use crate::subscriber::subscription::{SubKind, Subscription, SubscriptionMap, SubscriptionMeta};

pub trait SubscriptionMapper {
    fn map<Kind, ExchangeSub>(subscriptions: &[Subscription<Kind>]) -> SubscriptionMeta<Kind>
    where
        Kind: SubKind,
        ExchangeSub: ExchangeSubscription,
        Subscription<Kind>: Identifier<<ExchangeSub as ExchangeSubscription>::Channel>;
}

pub struct WebSocketSubMapper;

impl SubscriptionMapper for WebSocketSubMapper {
    fn map<Kind, ExchangeSub>(subscriptions: &[Subscription<Kind>]) -> SubscriptionMeta<Kind>
    where
        Kind: SubKind,
        ExchangeSub: ExchangeSubscription,
        Subscription<Kind>: Identifier<<ExchangeSub as ExchangeSubscription>::Channel>,
    {
        // Allocate SubscriptionIds HashMap to track identifiers for each actioned Subscription
        let mut subscription_map = SubscriptionMap(HashMap::with_capacity(subscriptions.len()));

        // Map Barter Subscriptions to exchange specific subscriptions
        let exchange_subs = subscriptions
            .iter()
            .map(|subscription| {
                // Translate Barter Subscription to exchange specific subscription
                let exchange_sub = ExchangeSub::new(subscription);

                // Determine the SubscriptionId associated with this exchange specific subscription
                let subscription_id = exchange_sub.id();

                // Use ExchangeSub SubscriptionId as the link to this Barter Subscription
                subscription_map
                    .0
                    .insert(subscription_id, subscription.clone());

                exchange_sub
            })
            .collect::<Vec<ExchangeSub>>();

        // Construct WebSocket message subscriptions requests
        let subscriptions = ExchangeSub::requests(exchange_subs);

        // Determine the expected number of SubResponses from the exchange in response
        let expected_responses = ExchangeSub::expected_responses(&subscription_map);

        SubscriptionMeta {
            map: subscription_map,
            expected_responses,
            subscriptions,
        }
    }
}