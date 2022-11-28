use self::{
    mapper::{SubscriptionMapper, WebSocketSubMapper},
    subscription::{ExchangeSubscription, SubKind, Subscription, SubscriptionIdentifier, SubscriptionMap, SubscriptionMeta},
    validator::{SubscriptionValidator, WebSocketSubValidator}
};
use barter_integration::{
    error::SocketError,
    protocol::websocket::{connect, WebSocket},
};
use futures::SinkExt;
use async_trait::async_trait;
use serde::Deserialize;
use std::marker::PhantomData;
use crate::exchange::ExchangeMeta;

/// Barter traits and data structures that support subscribing to exchange specific market data.
///
/// eg/ `struct Subscription`, `trait SubKind`, `trait ExchangeSubscription`, etc.
pub mod subscription;

/// Todo:
pub mod mapper;
pub mod validator;



/// Todo:
#[async_trait]
pub trait Subscriber<Exchange, Kind, ExchangeEvent> // Todo: Do we need ExchangeEvent here? Try without later
where
    Exchange: ExchangeMeta<ExchangeEvent>,
    Kind: SubKind,
    ExchangeEvent: SubscriptionIdentifier + for<'de> Deserialize<'de>,
{
    type SubMapper: SubscriptionMapper;
    type SubValidator: SubscriptionValidator;

    async fn subscribe(&self, subscriptions: &[Subscription<Kind>]) -> Result<(WebSocket, SubscriptionMap<Kind>), SocketError>;
}

/// Todo:
pub struct WebSocketSubscriber<Exchange, Kind, ExchangeEvent> {
    phantom: PhantomData<(Exchange, Kind, ExchangeEvent)>
}

#[async_trait]
impl<Exchange, Kind, ExchangeEvent> Subscriber<Exchange, Kind, ExchangeEvent> for WebSocketSubscriber<Exchange, Kind, ExchangeEvent>
where
    Exchange: ExchangeMeta<ExchangeEvent> + Sync,
    Kind: SubKind + Send + Sync,
    ExchangeEvent: SubscriptionIdentifier + for<'de> Deserialize<'de> + Sync,
{
    type SubMapper = WebSocketSubMapper;
    type SubValidator = WebSocketSubValidator;

    async fn subscribe(&self, subscriptions: &[Subscription<Kind>]) -> Result<(WebSocket, SubscriptionMap<Kind>), SocketError> {
        // Connect to exchange
        let mut websocket = connect(Exchange::base_url()).await?;

        // Map &[Subscription<Kind>] to SubscriptionMeta
        let SubscriptionMeta {
            map,
            subscriptions,
            expected_responses,
        } = Self::SubMapper::map::<Kind, Exchange::ExchangeSub, ExchangeEvent>(subscriptions);

        // Send Subscriptions
        for subscription in subscriptions {
            websocket.send(subscription).await?;
        }

        // Validate subscriptions
        let map = Self::SubValidator::validate::<Kind, <<Exchange as ExchangeMeta<ExchangeEvent>>::ExchangeSub as ExchangeSubscription<ExchangeEvent>>::SubResponse>(
            map, &mut websocket, expected_responses
        ).await?;

        Ok((websocket, map))
    }
}

impl<Exchange, Kind, ExchangeEvent> WebSocketSubscriber<Exchange, Kind, ExchangeEvent> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData::<(Exchange, Kind, ExchangeEvent)>::default()
        }
    }
}

