use self::{
    book::l1::BinanceOrderBookL1, channel::BinanceChannel, market::BinanceMarket,
    subscription::BinanceSubResponse, trade::BinanceTrade,
};
use crate::{
    exchange::{Connector, ExchangeId, ExchangeSub},
    subscriber::{validator::WebSocketSubValidator, WebSocketSubscriber},
    subscription::{book::OrderBooksL1, trade::PublicTrades, InstrumentMap},
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream, StreamSelector,
};
use barter_integration::{error::SocketError, protocol::websocket::WsMessage};
use std::{fmt::Debug, marker::PhantomData};
use url::Url;

/// Todo:
pub mod book;
pub mod channel;
pub mod futures;
pub mod market;
pub mod spot;
pub mod subscription;
pub mod trade;

/// Todo:
pub trait BinanceServer: Clone + Send {
    const ID: ExchangeId;
    fn websocket_url() -> &'static str;
    fn http_book_snapshot_url() -> &'static str;
}

/// Todo:
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Binance<Server> {
    server: PhantomData<Server>,
}

impl<Server> Connector for Binance<Server>
where
    Server: BinanceServer + Debug,
{
    const ID: ExchangeId = Server::ID;
    type Channel = BinanceChannel;
    type Market = BinanceMarket;
    type Subscriber = WebSocketSubscriber<Self::SubValidator>;
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

    fn expected_responses(_: &InstrumentMap) -> usize {
        1
    }
}

impl<Server> StreamSelector<PublicTrades> for Binance<Server>
where
    Server: BinanceServer + Debug + Send + Sync,
{
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, BinanceTrade>>;
}

impl<Server> StreamSelector<OrderBooksL1> for Binance<Server>
where
    Server: BinanceServer + Debug + Send + Sync,
{
    type Stream = ExchangeWsStream<StatelessTransformer<Self, OrderBooksL1, BinanceOrderBookL1>>;
}
