use crate::{
    model::{DataKind, PublicTrade},
    ExchangeId, Identifiable, MarketEvent, Validator,
};
use barter_integration::{
    error::SocketError,
    model::{Exchange, Instrument, Side, SubscriptionId},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `Ftx` message received in response to WebSocket subscription requests.
///
/// eg/ FtxResponse::Subscribed {"type": "subscribed", "channel": "trades", "market": "BTC/USDT"}
/// eg/ FtxResponse::Error {"type": "error", "code": 400, "msg": "Missing parameter \"channel\""}
///
/// See docs: <https://docs.ftx.com/#response-format>
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FtxSubResponse {
    Subscribed { channel: String, market: String },
    Error { msg: String },
}

impl Validator for FtxSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self {
            FtxSubResponse::Subscribed { .. } => Ok(self),
            FtxSubResponse::Error { msg } => Err(SocketError::Subscribe(format!(
                "received failure subscription response: {}",
                msg
            ))),
        }
    }
}

/// `Ftx` message variants that can be received over [`WebSocket`].
///
/// See docs: <https://docs.ftx.com/#public-channels>
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(tag = "channel", rename_all = "lowercase")]
pub enum FtxMessage {
    Trades {
        market: SubscriptionId,
        #[serde(rename = "data")]
        trades: Vec<FtxTrade>,
    },
}

impl Identifiable for FtxMessage {
    fn id(&self) -> SubscriptionId {
        match self {
            FtxMessage::Trades {
                market: subscription_id,
                ..
            } => subscription_id.clone(),
        }
    }
}

/// `Ftx` trade message.
///
/// See docs: <https://docs.ftx.com/#trades>
#[derive(Clone, Copy, PartialEq, Debug, Deserialize)]
pub struct FtxTrade {
    pub id: u64,
    pub price: f64,
    pub size: f64,
    pub side: Side,
    pub time: DateTime<Utc>,
}

impl From<(ExchangeId, Instrument, FtxTrade)> for MarketEvent {
    fn from((exchange, instrument, trade): (ExchangeId, Instrument, FtxTrade)) -> Self {
        Self {
            exchange_time: trade.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange.as_str()),
            instrument,
            kind: DataKind::Trade(PublicTrade {
                id: trade.id.to_string(),
                price: trade.price,
                quantity: trade.size,
                side: trade.side,
            }),
        }
    }
}
