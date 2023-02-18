use crate::{
    exchange::{
        bitmex::{
            channel::BitmexChannel, market::BitmexMarket, subscription::BitmexSubResponse,
            trade::BitmexTradePayload,
        },
        subscription::ExchangeSub,
        Connector, ExchangeId, ExchangeServer, StreamSelector,
    },
    subscriber::{validator::WebSocketSubValidator, WebSocketSubscriber},
    subscription::{trade::PublicTrades, Map},
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream,
};
use barter_integration::{error::SocketError, model::Instrument, protocol::websocket::WsMessage};
use serde::de::{Error, Unexpected};
use std::{fmt::Debug, marker::PhantomData};
use url::Url;

pub mod channel;

/// [`Subscription`](crate::subscription::Subscription) response type and response
/// [`Validator`](barter_integration::Validator) for [`Bitmex`].
pub mod subscription;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an exchange [`Connector`] specific market used for generating [`Connector::requests`].
mod market;

/// Generic [`BitmexMessage<T>`](message::BitmexMessage) type common to
/// [`BybitSpot`](spot::BybitSpot)
mod message;
mod trade;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Bitmex<Server> {
    server: PhantomData<Server>,
}

impl<Server> Connector for Bitmex<Server>
where
    Server: ExchangeServer,
{
    const ID: ExchangeId = Server::ID;
    type Channel = BitmexChannel;
    type Market = BitmexMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = BitmexSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(Server::websocket_url()).map_err(SocketError::UrlParse)
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        let stream_names = exchange_subs
            .into_iter()
            .map(|sub| format!("{}:{}", sub.channel.as_ref(), sub.market.as_ref(),))
            .collect::<Vec<String>>();

        vec![WsMessage::Text(
            serde_json::json!({
                "op": "subscribe",
                "args": stream_names
            })
            .to_string(),
        )]
    }

    fn expected_responses(_: &Map<Instrument>) -> usize {
        1
    }
}

impl<Server> StreamSelector<PublicTrades> for Bitmex<Server>
where
    Server: ExchangeServer + Debug + Send + Sync,
{
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, BitmexTradePayload>>;
}

impl<'de, Server> serde::Deserialize<'de> for Bitmex<Server>
where
    Server: ExchangeServer,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let input = <&str as serde::Deserialize>::deserialize(deserializer)?;
        let expected = Self::ID.as_str();

        if input == Self::ID.as_str() {
            Ok(Self::default())
        } else {
            Err(Error::invalid_value(Unexpected::Str(input), &expected))
        }
    }
}

impl<Server> serde::Serialize for Bitmex<Server>
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
