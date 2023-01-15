use self::{
    channel::BitfinexChannel, market::BitfinexMarket, message::BitfinexMessage,
    subscription::BitfinexPlatformEvent, validator::BitfinexWebSocketSubValidator,
};
use crate::{
    exchange::{Connector, ExchangeId, ExchangeSub, StreamSelector},
    subscriber::WebSocketSubscriber,
    subscription::trade::PublicTrades,
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream,
};
use barter_integration::{error::SocketError, protocol::websocket::WsMessage};
use barter_macro::{DeExchange, SerExchange};
use serde_json::json;
use url::Url;

/// Defines the type that translates a Barter [`Subscription`] into an exchange [`Connector`]
/// specific channel used for generating [`Connector::requests`].
pub mod channel;

/// Defines the type that translates a Barter [`Subscription`] into an exchange [`Connector`]
/// specific market used for generating [`Connector::requests`].
pub mod market;

/// [`BitfinexMessage`](message::BitfinexMessage) type for [`Bitfinex`].
pub mod message;

/// [`Subscription`] response types and response [`Validator`] for [`Bitfinex`].
pub mod subscription;

/// Public trade types for [`Bitfinex`].
pub mod trade;

/// Custom [`SubscriptionValidator`] implementation for [`Bitfinex`].
pub mod validator;

/// [`Bitfinex`] server base url.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
pub const BASE_URL_BITFINEX: &str = "wss://api-pub.bitfinex.com/ws/2";

/// [`Bitfinex`] exchange.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, DeExchange, SerExchange,
)]
pub struct Bitfinex;

impl Connector for Bitfinex {
    const ID: ExchangeId = ExchangeId::Bitfinex;
    type Channel = BitfinexChannel;
    type Market = BitfinexMarket;
    type Subscriber = WebSocketSubscriber;
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
