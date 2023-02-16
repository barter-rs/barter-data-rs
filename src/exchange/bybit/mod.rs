use crate::{
    exchange::{
        bybit::{
            channel::BybitChannel, market::BybitMarket, subscription::BybitSubResponse,
            trade::BybitTradePayload,
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
use std::{fmt::Debug, marker::PhantomData};
use url::Url;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an exchange [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// [`ExchangeServer`] and [`StreamSelector`] implementations for
/// [`BybitLinear`](futures::BybitFuturePerpetual).
pub mod futures;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an exchange [`Connector`] specific market used for generating [`Connector::requests`].
pub mod market;

/// Generic [`BybitMessage<T>`](message::BybitMessage) type common to
/// [`BybitSpot`](spot::BybitSpot)
pub mod message;

/// [`ExchangeServer`] and [`StreamSelector`] implementations for
/// [`BybitSpot`](spot::BybitSpot).
pub mod spot;
pub mod subscription;

/// Public trade types common to both [`BybitSpot`](spot::BybitSpot) and
/// [`BybitPerpetual`](futures::BybitPerpetual).
pub mod trade;

/// Generic [`Bybit<Server>`](Bybit) exchange.
///
/// ### Notes
/// A `Server` [`ExchangeServer`](super::ExchangeServer) implementations exists for
/// [`BybitSpot`](spot::BybitSpot) and [`BybitFuturePerpetual`](futures::BybitFuturePerpetual).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Bybit<Server> {
    server: PhantomData<Server>,
}

impl<Server> Connector for Bybit<Server>
where
    Server: ExchangeServer,
{
    const ID: ExchangeId = Server::ID;
    type Channel = BybitChannel;
    type Market = BybitMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = BybitSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(Server::websocket_url()).map_err(SocketError::UrlParse)
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        let stream_names = exchange_subs
            .into_iter()
            .map(|sub| format!("{}.{}", sub.channel.as_ref(), sub.market.as_ref(),))
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

impl<Server> StreamSelector<PublicTrades> for Bybit<Server>
where
    Server: ExchangeServer + Debug + Send + Sync,
{
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, BybitTradePayload>>;
}

impl<'de, Server> serde::Deserialize<'de> for Bybit<Server>
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

impl<Server> serde::Serialize for Bybit<Server>
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
