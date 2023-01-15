use self::{channel::GateioChannel, market::GateioMarket, subscription::GateioSubResponse};
use crate::{
    exchange::{subscription::ExchangeSub, Connector, ExchangeId, ExchangeServer},
    subscriber::{validator::WebSocketSubValidator, WebSocketSubscriber},
};
use barter_integration::{error::SocketError, protocol::websocket::WsMessage};
use serde_json::json;
use std::{fmt::Debug, marker::PhantomData};
use url::Url;

/// Defines the type that translates a Barter [`Subscription`] into an exchange [`Connector`]
/// specific channel used for generating [`Connector::requests`].
pub mod channel;
pub mod futures;

/// Defines the type that translates a Barter [`Subscription`] into an exchange [`Connector`]
/// specific market used for generating [`Connector::requests`].
pub mod market;

pub mod message;
pub mod spot;
pub mod subscription;

/// Todo:
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Gateio<Server> {
    server: PhantomData<Server>,
}

impl<Server> Connector for Gateio<Server>
where
    Server: ExchangeServer,
{
    const ID: ExchangeId = Server::ID;
    type Channel = GateioChannel;
    type Market = GateioMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = GateioSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(Server::websocket_url()).map_err(SocketError::UrlParse)
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        exchange_subs
            .into_iter()
            .map(|ExchangeSub { channel, market }| {
                WsMessage::Text(
                    json!({
                        "time": chrono::Utc::now().timestamp_millis(),
                        "channel": channel.as_ref(),
                        "event": "subscribe",
                        "payload": [market.as_ref()]
                    })
                    .to_string(),
                )
            })
            .collect()
    }
}

impl<'de, Server> serde::Deserialize<'de> for Gateio<Server>
where
    Server: ExchangeServer,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let input = <String as serde::Deserialize>::deserialize(deserializer)?;
        let expected = Self::ID.as_str();

        if input.as_str() == Self::ID.as_str() {
            Ok(Self::default())
        } else {
            Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(input.as_str()),
                &expected,
            ))
        }
    }
}

impl<Server> serde::Serialize for Gateio<Server>
where
    Server: ExchangeServer,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let exchange_id = Self::ID.as_str();
        serializer.serialize_str(exchange_id)
    }
}
