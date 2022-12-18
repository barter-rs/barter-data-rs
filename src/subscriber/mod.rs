use self::{
    mapper::{SubscriptionMapper, WebSocketSubMapper},
    subscription::{SubKind, Subscription, SubscriptionMap, SubscriptionMeta},
    validator::SubscriptionValidator,
};
use crate::{exchange::Connector, Identifier};
use async_trait::async_trait;
use barter_integration::{
    error::SocketError,
    protocol::websocket::{connect, WebSocket},
};
use futures::SinkExt;
use std::marker::PhantomData;
use tracing::{debug, info};

pub mod mapper;
/// Todo:
pub mod subscription;
pub mod validator;

/// Todo:
#[async_trait]
pub trait Subscriber<Validator>
where
    Validator: SubscriptionValidator,
{
    type SubMapper: SubscriptionMapper;

    async fn subscribe<Exchange, Kind>(
        subscriptions: &[Subscription<Exchange, Kind>],
    ) -> Result<(WebSocket, SubscriptionMap<Exchange, Kind>), SocketError>
    where
        Exchange: Connector + Send + Sync,
        Kind: SubKind + Send + Sync,
        Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
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

    async fn subscribe<Exchange, Kind>(
        subscriptions: &[Subscription<Exchange, Kind>],
    ) -> Result<(WebSocket, SubscriptionMap<Exchange, Kind>), SocketError>
    where
        Exchange: Connector + Send + Sync,
        Kind: SubKind + Send + Sync,
        Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
        Validator: 'async_trait,
    {
        // Connect to exchange
        let url = Exchange::base_url();
        let mut websocket = connect(url).await?;
        debug!(exchange = %Exchange::ID, %url, "connected to exchange WebSocket");

        // Map &[Subscription<Exchange, Kind>] to SubscriptionMeta
        let SubscriptionMeta {
            map,
            subscriptions,
            expected_responses,
        } = Self::SubMapper::map::<Exchange, Kind>(subscriptions);

        // Send Subscriptions
        for subscription in subscriptions {
            websocket.send(subscription).await?;
        }

        // Validate Subscriptions
        let map = Validator::validate::<Exchange, Kind>(map, &mut websocket, expected_responses).await?;


        info!(exchange = %Exchange::ID, %url, ?subscriptions, "subscribed to exchange WebSocket");
        Ok((websocket, map))
    }
}
