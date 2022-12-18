use self::{
    channel::CoinbaseChannel, market::CoinbaseMarket, subscription::CoinbaseSubResponse,
    trade::CoinbaseTrade,
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

/// [`Coinbase`] server base url.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview>
pub const BASE_URL_COINBASE: &str = "wss://ws-feed.exchange.coinbase.com";

/// [`Coinbase`] exchange.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Coinbase;

impl Connector for Coinbase {
    const ID: ExchangeId = ExchangeId::Coinbase;
    type Channel = CoinbaseChannel;
    type Market = CoinbaseMarket;
    type Subscriber = WebSocketSubscriber<Self::SubValidator>;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = CoinbaseSubResponse;

    fn base_url() -> &'static str {
        BASE_URL_COINBASE
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        exchange_subs
            .into_iter()
            .map(|ExchangeSub { channel, market }| {
                WsMessage::Text(
                    json!({
                        "type": "subscribe",
                        "product_ids": [market.0],
                        "channels": [channel.0],
                    })
                    .to_string(),
                )
            })
            .collect()
    }
}

impl StreamSelector<PublicTrades> for Coinbase {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, CoinbaseTrade>>;
}
