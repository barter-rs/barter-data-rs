use self::{
    book::l1::BinanceOrderBookL1, channel::BinanceChannel, market::BinanceMarket,
    subscription::BinanceSubResponse, trade::BinanceTrade,
};
use crate::{
    exchange::{Connector, ExchangeId, ExchangeServer, ExchangeSub},
    subscriber::{validator::WebSocketSubValidator, WebSocketSubscriber},
    subscription::{book::OrderBooksL1, trade::PublicTrades, Map},
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream, StreamSelector,
};
use barter_integration::{error::SocketError, model::Instrument, protocol::websocket::WsMessage};
use std::{fmt::Debug, marker::PhantomData};
use url::Url;

/// Todo:
pub mod book;

/// Defines the type that translates a Barter [`Subscription`] into an exchange [`Connector`]
/// specific channel used for generating [`Connector::requests`].
pub mod channel;

pub mod futures;

/// Defines the type that translates a Barter [`Subscription`] into an exchange [`Connector`]
/// specific market used for generating [`Connector::requests`].
pub mod market;

pub mod spot;
pub mod subscription;
pub mod trade;

/// Todo:
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Binance<Server> {
    server: PhantomData<Server>,
}

impl<Server> Connector for Binance<Server>
where
    Server: ExchangeServer,
{
    const ID: ExchangeId = Server::ID;
    type Channel = BinanceChannel;
    type Market = BinanceMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = BinanceSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(Server::websocket_url()).map_err(SocketError::UrlParse)
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        let stream_names = exchange_subs
            .into_iter()
            .map(|sub| {
                // Note:
                // Market must be lowercase when subscribing, but lowercase in general since
                // Binance sends message with uppercase MARKET (eg/ BTCUSDT).
                format!(
                    "{}{}",
                    sub.market.as_ref().to_lowercase(),
                    sub.channel.as_ref()
                )
            })
            .collect::<Vec<String>>();

        vec![WsMessage::Text(
            serde_json::json!({
                "method": "SUBSCRIBE",
                "params": stream_names,
                "id": 1
            })
            .to_string(),
        )]
    }

    fn expected_responses(_: &Map<Instrument>) -> usize {
        1
    }
}

impl<Server> StreamSelector<PublicTrades> for Binance<Server>
where
    Server: ExchangeServer + Debug + Send + Sync,
{
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, BinanceTrade>>;
}

impl<Server> StreamSelector<OrderBooksL1> for Binance<Server>
where
    Server: ExchangeServer + Debug + Send + Sync,
{
    type Stream = ExchangeWsStream<StatelessTransformer<Self, OrderBooksL1, BinanceOrderBookL1>>;
}

impl<'de, Server> serde::Deserialize<'de> for Binance<Server>
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

impl<Server> serde::Serialize for Binance<Server>
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
