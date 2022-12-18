use self::{
    channel::GateioChannel, market::GateioMarket, subscription::GateioSubResponse,
};
use crate::{
    exchange::{Connector, ExchangeId, ServerSelector},
    subscriber::{
        subscription::exchange::ExchangeSub,
        validator::WebSocketSubValidator,
        WebSocketSubscriber,
    },
};
use barter_integration::protocol::websocket::WsMessage;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, marker::PhantomData};
use serde_json::json;

/// Todo:
pub mod channel;
pub mod market;
pub mod subscription;
pub mod message;
pub mod futures;
pub mod spot;

/// Todo:
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize)]
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
