use crate::exchange::{Connector, ExchangeId, ExchangeSub, TransformerConstructor};
use crate::Identifier;
use crate::subscriber::subscription::{SubKind, Subscription, SubscriptionMap};
use crate::subscriber::subscription::trade::PublicTrades;
use crate::subscriber::validator::WebSocketSubValidator;
use crate::subscriber::WebSocketSubscriber;
use barter_integration::error::SocketError;
use barter_integration::model::SubscriptionId;
use barter_integration::protocol::websocket::WsMessage;
use barter_integration::Validator;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc;
use crate::exchange::coinbase::trade::CoinbaseTrade;
use crate::transformer::StatelessTransformer;

/// Todo:
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

impl<Kind> Connector<Kind> for Coinbase
where
    Kind: SubKind,
{
    const ID: ExchangeId = ExchangeId::Coinbase;
    type Channel = CoinbaseChannel;
    type Market = CoinbaseMarket;
    type Subscriber = WebSocketSubscriber<Self::SubValidator>;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = CoinbaseSubResponse;

    fn base_url() -> &'static str {
        BASE_URL_COINBASE
    }

    fn requests(sub_metas: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        sub_metas
            .into_iter()
            .map(|ExchangeSub { channel, market }| {
                WsMessage::Text(
                    json!({
                        "type": "subscribe",
                        "product_ids": [market.0],
                        "channels": [channel.0],
                    }).to_string(),
                )
            })
            .collect()
    }
}

impl TransformerConstructor<PublicTrades> for Coinbase {
    type T = StatelessTransformer<PublicTrades, CoinbaseTrade>;

    fn transformer(_: mpsc::UnboundedSender<WsMessage>, map: SubscriptionMap<PublicTrades>) -> Self::T {
        StatelessTransformer::new(<Coinbase as Connector<PublicTrades>>::ID, map)
    }
}

/// Todo:
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Debug, Copy, Clone)]
pub struct CoinbaseChannel(pub &'static str);

impl CoinbaseChannel {
    /// [`Coinbase`] real-time trades channel.
    ///
    /// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#match>
    pub const TRADES: Self = Self("matches");

    pub fn get_trades() -> String {
        todo!()
    }
}

impl Identifier<CoinbaseChannel> for Subscription<PublicTrades> {
    fn id(&self) -> CoinbaseChannel {
        CoinbaseChannel::TRADES
    }
}

/// Todo:
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Debug, Clone)]
pub struct CoinbaseMarket(pub String);

impl<Kind> Identifier<CoinbaseMarket> for Subscription<Kind> {
    fn id(&self) -> CoinbaseMarket {
        CoinbaseMarket(format!("{}-{}", self.instrument.base, self.instrument.quote).to_uppercase())
    }
}

/// Generate a [`Coinbase`] [`SubscriptionId`] from the channel and market provided.
///
/// Uses "channel|market":
/// eg/ SubscriptionId("matches|ETH-USD")
pub(crate) fn subscription_id(channel: CoinbaseChannel, market: &str) -> SubscriptionId {
    SubscriptionId::from(format!("{}|{}", channel.0, market))
}

/// Coinbase WebSocket subscription response.
///
/// eg/ CoinbaseResponse::Subscribed {"type": "subscriptions", "channels": [{"name": "matches", "product_ids": ["BTC-USD"]}]}
/// eg/ CoinbaseResponse::Error {"type": "error", "message": "error message", "reason": "reason"}
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CoinbaseSubResponse {
    #[serde(alias = "subscriptions")]
    Subscribed {
        channels: Vec<CoinbaseChannels>,
    },
    Error {
        reason: String,
    },
}

/// Communicates the Coinbase product_ids (eg/ "ETH-USD") associated with a successful channel
/// (eg/ "matches") subscription.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct CoinbaseChannels {
    #[serde(alias = "name")]
    pub channel: String,
    pub product_ids: Vec<String>,
}

impl Validator for CoinbaseSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self {
            CoinbaseSubResponse::Subscribed { .. } => Ok(self),
            CoinbaseSubResponse::Error { reason } => Err(SocketError::Subscribe(format!(
                "received failure subscription response: {}",
                reason
            ))),
        }
    }
}