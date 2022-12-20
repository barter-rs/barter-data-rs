use self::{
    mapper::{SubscriptionMapper, WebSocketSubMapper},
    validator::SubscriptionValidator,
};
use crate::{
    exchange::Connector,
    subscription::{SubKind, Subscription, SubscriptionMap, SubscriptionMeta},
    Identifier,
};
use async_trait::async_trait;
use barter_integration::{
    error::SocketError,
    protocol::websocket::{connect, WebSocket},
};
use futures::SinkExt;
use std::marker::PhantomData;
use tracing::{debug, info};

/// Todo:
pub mod mapper;
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
        // Define variables for logging ergonomics
        let exchange = Exchange::ID;
        let url = Exchange::base_url();
        debug!(%exchange, %url, ?subscriptions, "subscribing to WebSocket");

        // Connect to exchange
        let mut websocket = connect(url).await?;
        debug!(%exchange, %url, ?subscriptions, "connected to WebSocket");

        // Map &[Subscription<Exchange, Kind>] to SubscriptionMeta
        let SubscriptionMeta { map, subscriptions } =
            Self::SubMapper::map::<Exchange, Kind>(subscriptions);

        // Send Subscriptions over WebSocket
        for subscription in subscriptions {
            debug!(%exchange, %url, payload = ?subscription, "sending exchange subscription");
            websocket.send(subscription).await?;
        }

        // Validate Subscription responses
        let map = Validator::validate::<Exchange, Kind>(map, &mut websocket).await?;

        info!(%exchange, %url, "subscribed to WebSocket");
        Ok((websocket, map))
    }
}
