use std::marker::PhantomData;
use self::{
    subscription::{Subscription, SubscriptionMap},
    mapper::SubscriptionMapper,
};
use barter_integration::{error::SocketError, protocol::websocket::WebSocket};
use async_trait::async_trait;
use futures::SinkExt;
use barter_integration::protocol::websocket::connect;
use crate::exchange::Connector;
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
pub trait Subscriber<Validator>
where
    Validator: SubscriptionValidator,
{
    type SubMapper: SubscriptionMapper;

    async fn subscribe<Kind, Exchange>(
        subscriptions: &[Subscription<Kind>],
    ) -> Result<(WebSocket, SubscriptionMap<Kind>), SocketError>
    where
        Kind: SubKind + Send + Sync,
        Exchange: Connector,
        Subscription<Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
        Validator: 'async_trait;
}

pub struct WebSocketSubscriber<Validator> {
    phantom: PhantomData<Validator>,
}

#[async_trait]
impl<Validator> Subscriber<Validator> for WebSocketSubscriber<Validator>
where
    Validator: SubscriptionValidator,
{
    type SubMapper = WebSocketSubMapper;

    async fn subscribe<Kind, Exchange>(
        subscriptions: &[Subscription<Kind>],
    ) -> Result<(WebSocket, SubscriptionMap<Kind>), SocketError>
    where
        Kind: SubKind + Send + Sync,
        Exchange: Connector,
        Subscription<Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
        Validator: 'async_trait,
    {
        // Connect to exchange
        let mut websocket = connect(Exchange::base_url()).await?;

        // Map &[Subscription<Kind>] to SubscriptionMeta
        let SubscriptionMeta {
            map,
            subscriptions,
            expected_responses,
        } = Self::SubMapper::map::<Kind, Exchange>(subscriptions);

        // Send Subscriptions
        for subscription in subscriptions {
            websocket.send(subscription).await?;
        }

        // Validate Subscriptions
        let map = Validator::validate::<Kind, Exchange::SubResponse>(
            map,
            &mut websocket,
            expected_responses
        ).await?;

        Ok((websocket, map))
    }
}
