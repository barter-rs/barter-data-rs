use self::{
    mapper::{SubscriptionMapper, WebSocketSubMapper},
    validator::{SubscriptionValidator, WebSocketSubValidator},
    subscription::{Subscription, SubKind, SubscriptionMap, SubscriptionMeta, DomainSubscription, ExchangeMeta}
};
use barter_integration::{
    error::SocketError,
    model::SubscriptionId,
    protocol::websocket::{connect, WebSocket},
    Validator,
};
use futures::SinkExt;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;


/// Todo:
pub mod mapper;
pub mod validator;

/// Barter traits and data structures that support subscribing to exchange specific market data.
///
/// eg/ `Subscription`, `SubscriptionId`, `SubKind`, `DomainSubscription`, etc.
pub mod subscription;

/// Todo:
pub trait SubscriptionIdentifier {
    fn subscription_id(&self) -> SubscriptionId;
}

/// Todo:
#[async_trait]
pub trait Subscriber<Exchange, Kind, ExchangeEvent>
where
    Exchange: ExchangeMeta<ExchangeEvent>,
    Kind: SubKind,
    ExchangeEvent: SubscriptionIdentifier + DeserializeOwned,
{
    type SubMapper: SubscriptionMapper;
    type SubValidator: SubscriptionValidator;

    async fn subscribe(&self, subscriptions: &[Subscription<Exchange, Kind>]) -> Result<(WebSocket, SubscriptionMap<Kind>), SocketError>;
}

/// Todo:
pub struct WebSocketSubscriber<Exchange, Kind, ExchangeSubEvent> {
    pub phantom: PhantomData<(Exchange, Kind, ExchangeSubEvent)>
}

#[async_trait]
impl<Exchange, Kind, ExchangeEvent> Subscriber<Exchange, Kind, ExchangeEvent> for WebSocketSubscriber<Exchange, Kind, ExchangeEvent>
where
    Exchange: ExchangeMeta<ExchangeEvent> + Sync,
    Kind: SubKind + Send + Sync,
    ExchangeEvent: SubscriptionIdentifier + DeserializeOwned + Sync,
{
    type SubMapper = WebSocketSubMapper;
    type SubValidator = WebSocketSubValidator;

    async fn subscribe(&self, subscriptions: &[Subscription<Exchange, Kind>]) -> Result<(WebSocket, SubscriptionMap<Kind>), SocketError> {
        // Connect to exchange
        let mut websocket = connect(Exchange::base_url()).await?;

        // Map &[Subscription<Kind>] to SubscriptionMeta
        let SubscriptionMeta {
            map,
            subscriptions,
            expected_responses,
        } = Self::SubMapper::map::<Kind, Exchange::Sub, ExchangeEvent>(subscriptions); // Could be done without phantom if we have Kind & ExchangeSub

        // Send Subscriptions
        for subscription in subscriptions {
            websocket.send(subscription).await?;
        }

        // Validate subscriptions
        let map = Self::SubValidator::validate::<Kind, <<Exchange as ExchangeMeta<ExchangeEvent>>::Sub as DomainSubscription<ExchangeEvent>>::Response>(
            map, &mut websocket, expected_responses
        ).await?;

        Ok((websocket, map))
    }
}