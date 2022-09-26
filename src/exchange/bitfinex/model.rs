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
use chrono::{TimeZone, Utc};

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













/// [`Bitfinex`](super::Bitfinex) native non-aggregated trade message.
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct BitfinexTrade {
    /// Id given to the trade by the exchange.
    pub trade_id: u64,
    /// Millisecond timestamp recorded on the exchange.
    pub timestamp: u64,
    /// The amount of the trade, if negative a sell.
    pub amount: f64,
    /// The execution price of this trade.
    pub price: f64,
}

impl From<(ExchangeId, Instrument, BitfinexTrade)> for MarketEvent {
    fn from((exchange, instrument, trade): (ExchangeId, Instrument, BitfinexTrade)) -> Self {
        // TODO: Find a better way to convert ms timestamp to chrono::Utc
        let ts_secs = trade.timestamp / 1000;
        let ts_ns = (trade.timestamp % 1000) * 1_000_000;
        let exchange_time = Utc.timestamp(
            ts_secs.try_into().unwrap_or_default(),
            ts_ns.try_into().unwrap_or_default(),
        );
        let side = if trade.amount < 0.0 {
            Side::Sell
        } else {
            Side::Buy
        };
        Self {
            exchange_time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange.as_str()),
            instrument,
            kind: DataKind::Trade(crate::model::PublicTrade {
                id: trade.trade_id.to_string(),
                price: trade.price,
                quantity: trade.amount,
                side,
            }),
        }
    }
}

// TODO: Add error message
/// [`Bitfinex`](super::Bitfinex) message variants that can be received over [`WebSocket`](crate::WebSocket).
#[derive(Debug, Deserialize, Serialize)]
#[serde(from = "IntermediaryBitfinexMessage")]
pub enum BitfinexMessage {
    /// Heartbeat sent every 15 seconds
    HeartbeatEvent {
        subscription_id: SubscriptionId,
        msg: String,
    },
    /// Snapshot of recent trades initially sent over the socket.
    TradeSnapshotEvent {
        subscription_id: SubscriptionId,
        trades: Vec<BitfinexTrade>,
    },
    /// Update
    TradeUpdateEvent {
        subscription_id: SubscriptionId,
        update_type: String,
        trade: BitfinexTrade,
    },
    /// Sent in response to a socket connection, containing channel information
    SubscriptionEvent {
        // This will be in the form of trades|tBTCUSD as it is not formatted yet
        subscription_id: SubscriptionId,
        sub_response: TradingSubscriptionMessage,
    },
    InfoEvent {
        info: InfoMessage,
    }
}

// TODO: This transformation may be redundant
/// [`Bitfinex`](super::Bitfinex) raw message variants received over the connection to Bitfinex.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum IntermediaryBitfinexMessage {
    //SubscriptionEvent(TradingSubscriptionMessage),
    SubscriptionEvent {
        event: String,
        channel: String,
        #[serde(rename = "chanId")]
        chan_id: u32,
        symbol: String,
        pair: String
    },
    HeartbeatEvent(i32, String),
    TradesSnapshotEvent(i32, Vec<BitfinexTrade>),
    TradesUpdateEvent(i32, String, BitfinexTrade),
    InfoEvent(InfoMessage),
}

impl From<IntermediaryBitfinexMessage> for BitfinexMessage {
    fn from(inter_ev: IntermediaryBitfinexMessage) -> Self {
        println!("Intermed: {:?}", inter_ev);
        match inter_ev {
            IntermediaryBitfinexMessage::HeartbeatEvent(channel_num, msg) => {
                let subscription_id = SubscriptionId::from(channel_num.to_string());
                BitfinexMessage::HeartbeatEvent {
                    subscription_id,
                    msg,
                }
            }
            IntermediaryBitfinexMessage::TradesSnapshotEvent(channel_num, trades) => {
                let subscription_id = SubscriptionId::from(channel_num.to_string());
                BitfinexMessage::TradeSnapshotEvent {
                    subscription_id,
                    trades,
                }
            }
            IntermediaryBitfinexMessage::TradesUpdateEvent(channel_num, update_type, trade) => {
                let subscription_id = SubscriptionId::from(channel_num.to_string());
                BitfinexMessage::TradeUpdateEvent {
                    subscription_id,
                    update_type,
                    trade,
                }
            },
            IntermediaryBitfinexMessage::SubscriptionEvent { event, channel, chan_id, symbol, pair } => {
                //match &sub {
                    // BitfinexSubResponse::TradeSubscriptionMessage(trade_msg) => {
                        let subscription_id = Bitfinex::subscription_id(&channel.clone(), &pair.clone());
                        println!("SUBSCRIPTION ID: {}", subscription_id);
                        BitfinexMessage::SubscriptionEvent { 
                            subscription_id, 
                            sub_response: TradingSubscriptionMessage { event, channel, chan_id, symbol, pair }
                        }
                    // },
                    // BitfinexSubResponse::ErrorMessage (err) => {
                    //     // TODO: This is definitely wrong
                    //     let subscription_id = SubscriptionId::from("fwef");
                    //     BitfinexMessage::HeartbeatEvent { subscription_id, msg: "grs".to_string() }
                    // },
                //}
            },  
            IntermediaryBitfinexMessage::InfoEvent(info) => BitfinexMessage::InfoEvent { info },
        }
    }
}
