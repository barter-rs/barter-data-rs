use self::{
    channel::BinanceChannel, market::BinanceMarket, subscription::BinanceSubResponse,
    trade::BinanceTrade,
};
use crate::exchange::subscription::ExchangeSub;
use crate::{
    exchange::{Connector, ExchangeId, ServerSelector},
    subscriber::{validator::WebSocketSubValidator, WebSocketSubscriber},
    subscription::{trade::PublicTrades, SubscriptionMap},
    transformer::StatelessTransformer,
    ExchangeWsStream, StreamSelector,
};
use barter_integration::protocol::websocket::WsMessage;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, marker::PhantomData};

pub mod channel;
/// Todo:
pub mod futures;
pub mod market;
pub mod spot;
pub mod subscription;
pub mod trade;

/// Todo:
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize,
)]
pub struct Binance<Server> {
    server: PhantomData<Server>,
}

impl<Server> Connector for Binance<Server>
where
    Server: ServerSelector + Debug,
{
    const ID: ExchangeId = Server::ID;
    type Channel = BinanceChannel;
    type Market = BinanceMarket;
    type Subscriber = WebSocketSubscriber<Self::SubValidator>;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = BinanceSubResponse;

    fn base_url() -> &'static str {
        Server::base_url()
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

    fn expected_responses<Kind>(_: &SubscriptionMap<Self, Kind>) -> usize {
        1
    }
}

impl<Server> StreamSelector<PublicTrades> for Binance<Server>
where
    Server: ServerSelector + Debug + Send + Sync,
{
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, BinanceTrade>>;
}
