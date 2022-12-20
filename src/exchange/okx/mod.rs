use self::{
    channel::OkxChannel, market::OkxMarket, subscription::OkxSubResponse, trade::OkxTrades,
};
use crate::{
    exchange::{Connector, ExchangeId},
    ExchangeWsStream,
    StreamSelector,
    subscriber::{
        subscription::trade::PublicTrades,
        validator::WebSocketSubValidator,
        WebSocketSubscriber,
    }, transformer::StatelessTransformer,
};
use barter_integration::protocol::websocket::WsMessage;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::exchange::subscription::ExchangeSub;

pub mod channel;
/// Todo:
pub mod market;
pub mod subscription;
pub mod trade;

/// [`Okx`] server base url.
///
/// See docs: <https://www.okx.com/docs-v5/en/#overview-api-resources-and-support>
pub const BASE_URL_OKX: &str = "wss://wsaws.okx.com:8443/ws/v5/public";

/// Todo:
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Okx;

impl Connector for Okx {
    const ID: ExchangeId = ExchangeId::Okx;
    type Channel = OkxChannel;
    type Market = OkxMarket;
    type Subscriber = WebSocketSubscriber<Self::SubValidator>;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = OkxSubResponse;

    fn base_url() -> &'static str {
        BASE_URL_OKX
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        vec![WsMessage::Text(
            json!({
                "op": "subscribe",
                "args": &exchange_subs,
            })
            .to_string(),
        )]
    }
}

impl StreamSelector<PublicTrades> for Okx {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, OkxTrades>>;
}
