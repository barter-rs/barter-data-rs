use crate::{
    exchange::{subscription::ExchangeSub, Connector},
    subscription::{SubKind, Subscription, InstrumentMap, SubscriptionMeta},
    Identifier,
};
use barter_integration::model::SubscriptionId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Todo:
pub trait SubscriptionMapper {
    fn map<Exchange, Kind>(
        subscriptions: &[Subscription<Exchange, Kind>],
    ) -> SubscriptionMeta<Exchange, Kind>
    where
        Exchange: Connector,
        Kind: SubKind,
        Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>;
}

/// Todo:
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct WebSocketSubMapper;

impl SubscriptionMapper for WebSocketSubMapper {
    fn map<Exchange, Kind>(
        subscriptions: &[Subscription<Exchange, Kind>],
    ) -> SubscriptionMeta<Exchange, Kind>
    where
        Exchange: Connector,
        Kind: SubKind,
        Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
        ExchangeSub<Exchange::Channel, Exchange::Market>: Identifier<SubscriptionId>,
    {
        // Allocate SubscriptionIds HashMap to track identifiers for each actioned Subscription
        let mut subscription_map = InstrumentMap(HashMap::with_capacity(subscriptions.len()));

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
            .collect::<Vec<ExchangeSub<Exchange::Channel, Exchange::Market>>>();

        // Construct WebSocket message subscriptions requests
        let subscriptions = Exchange::requests(exchange_subs);

        SubscriptionMeta {
            map: subscription_map,
            subscriptions,
        }
    }
}
