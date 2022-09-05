use super::KrakenFuturesUsd;
use crate::{
    model::{DataKind, PublicTrade, SubKind},
    ExchangeId, ExchangeTransformer, MarketEvent,
};
use barter_integration::{
    error::SocketError,
    model::{Exchange, Instrument, Side, SubscriptionId},
    protocol::websocket::WsMessage,
    Validator,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, Serializer};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct KrakenFuturesUsdSubscription {
    pub event: &'static str,
    #[serde(rename = "feed")]
    pub kind: KrakenFuturesUsdSubKind,
    #[serde(rename = "product_ids", serialize_with = "str_to_vec_serialize")]
    pub pair: String,
}

fn str_to_vec_serialize<S>(x: &str, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.collect_seq(vec![x])
}

impl TryFrom<&KrakenFuturesUsdSubscription> for WsMessage {
    type Error = SocketError;

    fn try_from(kraken_sub: &KrakenFuturesUsdSubscription) -> Result<Self, Self::Error> {
        serde_json::to_string(&kraken_sub)
            .map(WsMessage::text)
            .map_err(|error| SocketError::Serde {
                error,
                payload: format!("{kraken_sub:?}"),
            })
    }
}

impl KrakenFuturesUsdSubscription {
    const EVENT: &'static str = "subscribe";

    /// Construct a new [`KrakenSubscription`] from the provided pair (eg/ "XBT/USD") and
    /// [`KrakenSubKind`].
    pub fn new(pair: String, kind: KrakenFuturesUsdSubKind) -> Self {
        Self {
            event: Self::EVENT,
            pair,
            kind,
        }
    }
}

impl From<&KrakenFuturesUsdSubscription> for SubscriptionId {
    fn from(kraken_subscription: &KrakenFuturesUsdSubscription) -> Self {
        match kraken_subscription.kind {
            KrakenFuturesUsdSubKind::Trade(trade) => {
                // eg/ SubscriptionId::from("trade|XBT/USD")
                SubscriptionId::from(format!("{trade}|{}", kraken_subscription.pair))
            }
        }
    }
}

/// Possible [`KrakenSubscription`] variants.
///
/// eg/ KrakenSubKind::Candle {
///         "channel": "ohlc",
///         "interval": "5",
/// }
/// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
#[serde(untagged)]
pub enum KrakenFuturesUsdSubKind {
    Trade(&'static str),
}

impl KrakenFuturesUsdSubKind {
    const TRADE: &'static str = "trade";
}

impl TryFrom<&SubKind> for KrakenFuturesUsdSubKind {
    type Error = SocketError;

    fn try_from(kind: &SubKind) -> Result<Self, Self::Error> {
        match kind {
            SubKind::Trade => Ok(KrakenFuturesUsdSubKind::Trade(
                KrakenFuturesUsdSubKind::TRADE,
            )),
            other => Err(SocketError::Unsupported {
                entity: KrakenFuturesUsd::EXCHANGE.as_str(),
                item: other.to_string(),
            }),
        }
    }
}

/// `KrakenFutures` message received in response to WebSocket subscription requests.
///
/// eg/ KrakenFuturesSubResponse::Subscribed {"event":"subscribed","feed":"trade","product_ids":["PI_XBTUSD",]}
/// eg/ KrakenFuturesSubResponse::Error
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum KrakenFuturesUsdSubResponse {
    Subscribed {
        feed: String,
        product_ids: Vec<String>,
    },
    Error(KrakenFuturesUsdError),
}

impl Validator for KrakenFuturesUsdSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self {
            KrakenFuturesUsdSubResponse::Subscribed { .. } => Ok(self),
            KrakenFuturesUsdSubResponse::Error(error) => Err(SocketError::Subscribe(format!(
                "received failure subscription response: {}",
                error.message
            ))),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct KrakenFuturesUsdError {
    #[serde(rename = "errorMessage")]
    pub message: String,
}

/// `KrakenFutures` Message variants that can be received over [`WebSocket`].
/// `KrakenFutures` sends payloads as untagged array of arrays. Using a tuple
/// enum for trades to help serde deserialize messages properly.
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(tag = "feed")]
pub enum KrakenFuturesUsdMessage {
    #[serde(rename = "trade_snapshot")]
    TradeSnapshot {
        product_id: SubscriptionId,
        trades: Vec<KrakenFuturesUsdTrade>,
    },
    #[serde(rename = "trade")]
    Trade(KrakenFuturesUsdTrade),
}

/// Collection of [`KrakenTrade`] items with an associated [`SubscriptionId`] (eg/ "trade|XBT/USD").
///
/// See docs: <https://docs.kraken.com/websockets/#message-trade>
#[derive(Clone, PartialEq, PartialOrd, Debug, Serialize)]
pub struct KrakenFuturesUsdTrades {
    pub subscription_id: SubscriptionId,
    pub trades: Vec<KrakenFuturesUsdTrade>,
}

/// `KrakenFutures` trade message.
#[derive(Clone, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
pub struct KrakenFuturesUsdTrade {
    pub product_id: SubscriptionId,
    pub uid: String,
    pub side: Side,
    #[serde(rename = "type")]
    pub fill_type: String,
    pub seq: u64,
    pub time: DateTime<Utc>,
    #[serde(rename = "qty")]
    pub quantity: f64,
    pub price: f64,
}

impl From<(ExchangeId, Instrument, KrakenFuturesUsdTrade)> for MarketEvent {
    fn from(
        (exchange_id, instrument, trade): (ExchangeId, Instrument, KrakenFuturesUsdTrade),
    ) -> Self {
        Self {
            exchange_time: trade.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::Trade(PublicTrade {
                id: trade.uid,
                price: trade.price,
                quantity: trade.quantity,
                side: trade.side,
            }),
        }
    }
}
