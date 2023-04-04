use self::{
    channel::PoloniexChannel, market::PoloniexMarket, subscription::PoloniexSubResponse, trade::PoloniexTrade,
};
use crate::{
    exchange::{Connector, ExchangeId, ExchangeSub, StreamSelector},
    subscriber::{validator::WebSocketSubValidator, WebSocketSubscriber},
    subscription::trade::PublicTrades,
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream,
};
use barter_integration::{error::SocketError, protocol::websocket::WsMessage};
use barter_macro::{DeExchange, SerExchange};
use serde_json::json;
use url::Url;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an exchange [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an exchange [`Connector`] specific market used for generating [`Connector::requests`].
pub mod market;

/// [`Subscription`](crate::subscription::Subscription) response type and response
/// [`Validator`](barter_integration::Validator) for [`Okx`].
pub mod subscription;

/// Generic [`PoloniexMessage<T>`](message::PoloniexMessage)
pub mod message;

/// Public trade types for [`Poloniex`]
pub mod trade;

pub const BASE_URL_POLONIEX: &str = "wss://ws.poloniex.com/ws/public";

/// [`Poloniex`] exchange.
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api>
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, DeExchange, SerExchange,
)]
pub struct Poloniex;

impl Connector for Poloniex {
    const ID: ExchangeId = ExchangeId::Poloniex;
    type Channel = PoloniexChannel;
    type Market = PoloniexMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = PoloniexSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(BASE_URL_POLONIEX).map_err(SocketError::UrlParse)
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        exchange_subs
            .into_iter()
            .map(|ExchangeSub { channel, market }| {
                WsMessage::Text(
                    json!({
                        "event": "subscribe",
                        "channel": [channel.as_ref()],
                        "symbols": [market.as_ref()]
                    })
                    .to_string(),
                )
            })
            .collect()
    }
}


impl StreamSelector<PublicTrades> for Poloniex {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, PoloniexTrade>>;
}