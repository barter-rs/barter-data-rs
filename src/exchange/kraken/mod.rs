use self::{
    channel::KrakenChannel,
    market::KrakenMarket,
    subscription::KrakenSubResponse,
    message::KrakenMessage
};
use crate::{
    exchange::{Connector, ExchangeId},
    subscriber::{
        subscription::{exchange::ExchangeSub, trade::PublicTrades},
        validator::WebSocketSubValidator,
        WebSocketSubscriber,
    },
    transformer::StatelessTransformer,
    ExchangeWsStream, StreamSelector,
};
use barter_integration::protocol::websocket::WsMessage;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Todo:
pub mod channel;
pub mod market;
pub mod subscription;
pub mod trade;
pub mod message;

/// [`Kraken`] server base url.
///
/// See docs: <https://docs.kraken.com/websockets/#overview>
pub const BASE_URL_KRAKEN: &str = "wss://ws.kraken.com/";

/// [`Kraken`] exchange.
///
/// See docs: <https://docs.kraken.com/websockets/#overview>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Kraken;

impl Connector for Kraken {
    const ID: ExchangeId = ExchangeId::BinanceFuturesUsd;
    type Channel = KrakenChannel;
    type Market = KrakenMarket;
    type Subscriber = WebSocketSubscriber<Self::SubValidator>;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = KrakenSubResponse;

    fn base_url() -> &'static str {
        BASE_URL_KRAKEN
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        exchange_subs
            .into_iter()
            .map(|ExchangeSub { channel, market }| {
                WsMessage::Text(
                    json!({
                        "event": "subscribe",
                        "pair": [market.as_ref()],
                        "subscription": {
                            "name": channel.as_ref()
                        }
                    })
                    .to_string(),
                )
            })
            .collect()
    }
}

impl StreamSelector<PublicTrades> for Kraken {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, KrakenMessage>>;
}