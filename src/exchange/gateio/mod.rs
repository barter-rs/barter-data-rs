use self::{channel::GateioChannel, market::GateioMarket, subscription::GateioSubResponse};
use crate::exchange::subscription::ExchangeSub;
use crate::{
    exchange::{Connector, ExchangeId, ServerSelector},
    subscriber::{validator::WebSocketSubValidator, WebSocketSubscriber},
};
use barter_integration::protocol::websocket::WsMessage;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fmt::Debug, marker::PhantomData};

/// Todo:
pub mod channel;
pub mod futures;
pub mod market;
pub mod message;
pub mod spot;
pub mod subscription;

/// Todo:
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize,
)]
pub struct Gateio<Server> {
    server: PhantomData<Server>,
}

impl<Server> Connector for Gateio<Server>
where
    Server: ServerSelector + Debug,
{
    const ID: ExchangeId = Server::ID;
    type Channel = GateioChannel;
    type Market = GateioMarket;
    type Subscriber = WebSocketSubscriber<Self::SubValidator>;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = GateioSubResponse;

    fn base_url() -> &'static str {
        Server::base_url()
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
