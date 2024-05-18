use crate::instrument::InstrumentData;
use crate::{
    exchange::{subscription::ExchangeSub, Connector},
    subscription::{Map, SubKind, Subscription, SubscriptionMeta},
    Identifier,
};
use barter_integration::model::SubscriptionId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Defines how to map a collection of Barter [`Subscription`]s into exchange specific
/// [`SubscriptionMeta`], containing subscription payloads that are sent to the exchange.
pub trait SubscriptionMapper {
    fn map<Exchange, Instrument, Kind>(
        subscriptions: &[Subscription<Exchange, Instrument, Kind>],
    ) -> SubscriptionMeta<Instrument::Id>
    where
        Exchange: Connector,
        Instrument: InstrumentData,
        Kind: SubKind,
        Subscription<Exchange, Instrument, Kind>:
            Identifier<Exchange::Channel> + Identifier<Exchange::Market>;
}

/// Standard [`SubscriptionMapper`] for
/// [`WebSocket`](barter_integration::protocol::websocket::WebSocket)s suitable for most exchanges.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct WebSocketSubMapper;

impl SubscriptionMapper for WebSocketSubMapper {
    fn map<Exchange, Instrument, Kind>(
        subscriptions: &[Subscription<Exchange, Instrument, Kind>],
    ) -> SubscriptionMeta<Instrument::Id>
    where
        Exchange: Connector,
        Kind: SubKind,
        Instrument: InstrumentData,
        Subscription<Exchange, Instrument, Kind>:
            Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
        ExchangeSub<Exchange::Channel, Exchange::Market>: Identifier<SubscriptionId>,
    {
        // Allocate SubscriptionIds HashMap to track identifiers for each actioned Subscription
        let mut instrument_map = Map(HashMap::with_capacity(subscriptions.len()));

        // Map Barter Subscriptions to exchange specific subscriptions
        let exchange_subs = subscriptions
            .iter()
            .map(|subscription| {
                // Translate Barter Subscription to exchange specific subscription
                let exchange_sub = ExchangeSub::new(subscription);

                // Determine the SubscriptionId associated with this exchange specific subscription
                let subscription_id = exchange_sub.id();

                // Use ExchangeSub SubscriptionId as the link to this Barter Subscription
                instrument_map
                    .0
                    .insert(subscription_id, subscription.instrument.id().clone());

                exchange_sub
            })
            .collect::<Vec<ExchangeSub<Exchange::Channel, Exchange::Market>>>();

        // Construct WebSocket message subscriptions requests
        let subscriptions = Exchange::requests(exchange_subs);

        SubscriptionMeta {
            instrument_map,
            subscriptions: subscriptions.into_iter().collect(),
        }
    }
}
