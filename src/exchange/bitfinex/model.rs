use barter_integration::error;
use crate::{
    model::{DataKind, MarketEvent},
    ExchangeId,
};
use barter_integration::{
    model::{Exchange, Instrument, Side, SubscriptionId},
    error::SocketError,
    Validator,
};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, TimeZone, Utc};
use crate::model::PublicTrade;

/// [`Bitfinex`](super::Bitfinex) message variants received in response to WebSocket
/// subscription requests.
///
/// ## Examples
/// Trade Subscription Response
/// ``` json
/// {
///   event: "subscribed",
///   channel: "trades",
///   chanId: CHANNEL_ID,
///   symbol: "tBTCUSD"
///   pair: "BTCUSD"
/// }
/// ```
///
/// Candle Subscription Response
/// ``` json
/// {
///   event: "subscribed",
///   channel: "candles",
///   chanId: CHANNEL_ID,
///   key: "trade:1m:tBTCUSD"
/// }
/// ```
///
/// Error Subscription Response
/// ``` json
/// {
///    "event": "error",
///    "msg": ERROR_MSG,
///    "code": ERROR_CODE
/// }
/// ```
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
#[serde(tag = "event", rename_all = "lowercase")]
pub enum BitfinexSubResponse {
    Subscribed(BitfinexSubResponseKind),
    Error(BitfinexError),
}

/// [`Bitfinex`](super::Bitfinex) subscription success response variants for each channel.
///
/// ## Examples
/// Trade Subscription Response
/// ``` json
/// {
///   event: "subscribed",
///   channel: "trades",
///   chanId: CHANNEL_ID,
///   symbol: "tBTCUSD"
///   pair: "BTCUSD"
/// }
/// ```
///
/// Candle Subscription Response
/// ``` json
/// {
///   event: "subscribed",
///   channel: "candles",
///   chanId: CHANNEL_ID,
///   key: "trade:1m:tBTCUSD"
/// }
/// ```
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
#[serde(tag = "channel", rename_all = "lowercase")]
enum BitfinexSubResponseKind {
    Trades {
        #[serde(rename = "chanId")]
        channel_id: u32,
        #[serde(rename = "symbol")]
        market: String,
    },
    Candles {
        #[serde(rename = "chanId")]
        channel_id: u32,
        key: String,
    }
}

/// [`Bitfinex`](super::Bitfinex) error message that is received if a [`BitfinexSubResponse`]
/// indicates a WebSocket subscription failure.
///
/// ## Subscription Error Codes:
/// 10300: Generic failure
/// 10301: Already subscribed
/// 10302: Unknown channel
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Serialize)]
pub struct BitfinexError {
    msg: String,
    code: u32,
}

impl Validator for BitfinexSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self {
            BitfinexSubResponse::Subscribed { .. } => Ok(self),
            BitfinexSubResponse::Error(error) => Err(SocketError::Subscribe(format!(
                "received failure subscription response code: {} with message: {}",
                error.code, error.msg,
            ))),
        }
    }
}

impl Validator for BitfinexSubResponse {
    fn validate(self) -> Result<Self, barter_integration::error::SocketError>
    where
        Self: Sized,
    {
        match &self {
            BitfinexSubResponse::TradeSubscriptionMessage(_) => Ok(self),
            BitfinexSubResponse::ErrorMessage (_)  => {
                Err(barter_integration::error::SocketError::Subscribe(format!(
                    "received failed subscription response: ",
                )))
            }
        }
    }
}

/// [`Bitfinex`](super::Bitfinex) platform status message containing the server we are connecting
/// to, the version of the API, and if it is in maintenance mode.
///
/// ## Example
/// Platform Status Online
/// ``` json
/// {
///   "event": "info",
///   "version":  VERSION,
///   "platform": {
///     "status": 1
///   }
/// }
/// ```
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general#info-messages>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize)]
pub struct BitfinexPlatformStatus {
    version: u8,
    #[serde(rename = "serverId")]
    server_id: String,
    #[serde(rename = "platform")]
    status: Status,
}

/// [`Bitfinex`](super::Bitfinex) platform [`Status`] indicating if the API is in maintenance mode.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general#info-messages>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Status {
    Maintenance,
    Operative,
}

impl Validator for BitfinexPlatformStatus {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match self.status {
            Status::Operative => Ok(self),
            Status::Maintenance => Err(SocketError::Subscribe(format!(
                "exchange version: {} with server_id: {} is in maintenance mode",
                self.version, self.server_id,
            ))),
        }
    }
}

/// [`Bitfinex`](super::Bitfinex) message received over [`WebSocket`](crate::WebSocket) relating
/// to an active [`Subscription`](crate::Subscription). The message is associated with the original
/// [`Subscription`](crate::Subscription) using the `channel_id` field as the
/// [`SubscriptionId`](crate::SubscriptionId).
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BitfinexMessage {
    channel_id: u32,
    payload: BitfinexPayload,
}

/// [`Bitfinex`](super::Bitfinex) market data variants associated with an
/// active [`Subscription`](crate::Subscription).
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
#[derive(Clone, Copy, PartialEq, Debug, Deserialize)]
pub enum BitfinexPayload {
    Heartbeat,
    Trade(BitfinexTrade),
}


/// [`Bitfinex`](super::Bitfinex) real-time trade message.
///
/// Format: \[ID, TIME, AMOUNT, PRICE\], where +/- of amount indicates Side
/// eg/ \[401597395,1574694478808,0.005,7245.3\]
///
/// ## Notes:
/// - [`Bitfinex`](super::Bitfinex) trades subscriptions results in receiving tag="te" & tag="tu"
/// trades, both of which are identical.
/// - "te" trades arrive marginally faster.
/// - Therefore, tag="tu" trades are filtered out and considered only as additional Heartbeats.
///
/// See docs: <https://docs.bitfinex.com/reference/ws-public-trades>
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct BitfinexTrade {
    /// Exchange trade identifier.
    pub id: u64,
    /// Exchange timestamp for trade.
    pub time: DateTime<Utc>,
    /// Side of the trade: buy or sell.
    pub side: Side,
    /// Trade execution price.
    pub price: f64,
    /// Trade quantity.
    pub quantity: f64,
}

impl From<(ExchangeId, Instrument, BitfinexTrade)> for MarketEvent {
    fn from((exchange_id, instrument, trade): (ExchangeId, Instrument, BitfinexTrade)) -> Self {
        Self {
            exchange_time: trade.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: DataKind::Trade(PublicTrade {
                id: trade.id.to_string(),
                price: trade.price,
                quantity: trade.quantity,
                side: trade.side,
            }),
        }
    }
}