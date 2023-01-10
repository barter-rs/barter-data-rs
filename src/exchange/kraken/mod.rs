use self::{
    channel::KrakenChannel, market::KrakenMarket, message::KrakenMessage,
    subscription::KrakenSubResponse,
};
use crate::{
    exchange::{Connector, ExchangeId, ExchangeSub},
    subscriber::{validator::WebSocketSubValidator, WebSocketSubscriber},
    subscription::trade::PublicTrades,
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream, StreamSelector,
};
use barter_integration::{error::SocketError, protocol::websocket::WsMessage};
use barter_macro::{DeExchange, SerExchange};
use serde_json::json;
use url::Url;

/// Todo:
pub mod channel;
pub mod market;
pub mod message;
pub mod subscription;
pub mod trade;

/// [`Kraken`] server base url.
///
/// See docs: <https://docs.kraken.com/websockets/#overview>
pub const BASE_URL_KRAKEN: &str = "wss://ws.kraken.com/";

/// [`Kraken`] exchange.
///
/// See docs: <https://docs.kraken.com/websockets/#overview>
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, DeExchange, SerExchange,
)]
pub struct Kraken;

impl Connector for Kraken {
    const ID: ExchangeId = ExchangeId::Kraken;
    type Channel = KrakenChannel;
    type Market = KrakenMarket;
    type Subscriber = WebSocketSubscriber<Self::SubValidator>;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = KrakenSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(BASE_URL_KRAKEN).map_err(SocketError::UrlParse)
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
