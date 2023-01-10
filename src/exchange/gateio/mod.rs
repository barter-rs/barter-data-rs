use self::{channel::GateioChannel, market::GateioMarket, subscription::GateioSubResponse};
use crate::{
    exchange::{subscription::ExchangeSub, Connector, ExchangeId},
    subscriber::{validator::WebSocketSubValidator, WebSocketSubscriber},
};
use barter_integration::{error::SocketError, protocol::websocket::WsMessage};
use serde_json::json;
use std::{fmt::Debug, marker::PhantomData};
use url::Url;

/// Todo:
pub mod channel;
pub mod futures;
pub mod market;
pub mod message;
pub mod spot;
pub mod subscription;

/// Todo:
pub trait GateioServer: Default + Debug + Clone + Send {
    const ID: ExchangeId;
    fn websocket_url() -> &'static str;
}

/// Todo:
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Gateio<Server> {
    server: PhantomData<Server>,
}

impl<Server> Connector for Gateio<Server>
where
    Server: GateioServer + Debug,
{
    const ID: ExchangeId = Server::ID;
    type Channel = GateioChannel;
    type Market = GateioMarket;
    type Subscriber = WebSocketSubscriber<Self::SubValidator>;
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
