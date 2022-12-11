use std::marker::PhantomData;
use self::{
    subscription::{Subscription, SubscriptionMap},
    mapper::SubscriptionMapper,
};
use barter_integration::{error::SocketError, protocol::websocket::WebSocket};
use async_trait::async_trait;
use futures::SinkExt;
use barter_integration::protocol::websocket::connect;
use crate::exchange::{Exchange, ExchangeSubscription};
use crate::Identifier;
use crate::subscriber::mapper::WebSocketSubMapper;
use crate::subscriber::subscription::{SubKind, SubscriptionMeta};
use crate::subscriber::validator::SubscriptionValidator;

/// Todo:
pub mod subscription;
pub mod mapper;
pub mod validator;

/// Todo:
#[async_trait]
pub trait Subscriber<Ex, Validator>
where
    Ex: Exchange,
{
    type SubMapper: SubscriptionMapper;

    async fn subscribe<Kind>(
        &self,
        subscriptions: &[Subscription<Kind>],
    ) -> Result<(WebSocket, SubscriptionMap<Kind>), SocketError>
    where
        Kind: SubKind + Send + Sync,
        Subscription<Kind>: Identifier<<<Ex as Exchange>::Sub as ExchangeSubscription>::Channel>;
}

// Todo: These generics could be replace by generics on subscribe
//  '--> would require caller to define them -> Validator could assoc type of Ex? eek.
pub struct WebSocketSubscriber<Ex, Validator> {
    phantom: PhantomData<(Ex, Validator)>,
}

#[async_trait]
impl<Ex, Validator> Subscriber<Ex, Validator> for WebSocketSubscriber<Ex, Validator>
where
    Ex: Exchange + Sync,
    Validator: SubscriptionValidator + Sync,
{
    type SubMapper = WebSocketSubMapper;

    async fn subscribe<Kind>(&self, subscriptions: &[Subscription<Kind>]) -> Result<(WebSocket, SubscriptionMap<Kind>), SocketError>
    where
        Kind: SubKind + Send + Sync,
        Subscription<Kind>: Identifier<<<Ex as Exchange>::Sub as ExchangeSubscription>::Channel>,
    {
        // Connect to exchange
        let mut websocket = connect(Ex::base_url()).await?;

        // Map &[Subscription<Kind>] to SubscriptionMeta
        let SubscriptionMeta {
            map,
            subscriptions,
            expected_responses,
        } = Self::SubMapper::map::<Kind, Ex::Sub>(subscriptions);

        // Send Subscriptions
        for subscription in subscriptions {
            websocket.send(subscription).await?;
        }

        // Validate Subscriptions
        let map = Validator::validate::<
            Kind,
            <<Ex as Exchange>::Sub as ExchangeSubscription>::SubResponse
        >(
            map,
            &mut websocket,
            expected_responses
        ).await?;

        Ok((websocket, map))
    }
}

impl<Ex, Validator> Default for WebSocketSubscriber<Ex, Validator> {
    fn default() -> Self {
        Self {
            phantom: PhantomData::<(Ex, Validator)>::default(),
        }
    }
}

impl<Ex, Validator> WebSocketSubscriber<Ex, Validator> {
    pub fn new() -> Self {
        Self::default()
    }
}