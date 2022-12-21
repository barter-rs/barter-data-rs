use self::{
    channel::BitfinexChannel, market::BitfinexMarket, message::BitfinexMessage,
    subscription::BitfinexPlatformEvent, validator::BitfinexWebSocketSubValidator,
};
use crate::{
    exchange::{Connector, ExchangeId, ExchangeSub},
    subscriber::WebSocketSubscriber,
    subscription::trade::PublicTrades,
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream, StreamSelector,
};
use barter_integration::{error::SocketError, protocol::websocket::WsMessage};
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;

/// Todo:
pub mod channel;
pub mod market;
pub mod message;
pub mod subscription;
pub mod trade;
pub mod validator;

/// [`Bitfinex`] server base url.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
pub const BASE_URL_BITFINEX: &str = "wss://api-pub.bitfinex.com/ws/2";

/// [`Bitfinex`] exchange.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Bitfinex;

impl Connector for Bitfinex {
    const ID: ExchangeId = ExchangeId::Bitfinex;
    type Channel = BitfinexChannel;
    type Market = BitfinexMarket;
    type Subscriber = WebSocketSubscriber<Self::SubValidator>;
    type SubValidator = BitfinexWebSocketSubValidator;
    type SubResponse = BitfinexPlatformEvent;

    fn url() -> Result<Url, SocketError> {
        Url::parse(BASE_URL_BITFINEX).map_err(SocketError::UrlParse)
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        exchange_subs
            .into_iter()
            .map(|ExchangeSub { channel, market }| {
                WsMessage::Text(
                    json!({
                        "event": "subscribe",
                        "channel": channel.as_ref(),
                        "symbol": market.as_ref(),
                    })
                    .to_string(),
                )
            })
            .collect()
    }
}

impl StreamSelector<PublicTrades> for Bitfinex {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, BitfinexMessage>>;
}
