use barter_integration::{
    model::{Exchange, Instrument, Side, SubscriptionId},
    Validator,
};
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    model::{DataKind, MarketEvent},
    ExchangeId,
};

use super::Bitfinex;


/// [`Bitfinex`] message received in response to WebSocket subscription request.
///
/// ## Error types
/// 10300 : Subscription failed (generic)
/// 10301 : Already subscribed
/// 10302 : Unknown channel
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BitfinexSubResponse {
    /// Response to successful subscription request to trade stream
    TradeSubscriptionMessage {
        event: String,
        channel: String,
        #[serde(rename = "chanId")]
        channel_id: u32,
        symbol: String,
        pair: String,

    },
    // TODO: Make sure they are all formatted like this
    /// Erroneous subscription request
    Error {
        event: String,
        msg: String,
        // TODO: Change this to map to specific error value
        code: u32,
        chan_id: Option<u32>,
        symbol: Option<String>,
        channel: Option<String>,
    },
}

impl Validator for BitfinexSubResponse {
    fn validate(self) -> Result<Self, barter_integration::error::SocketError>
    where
        Self: Sized,
    {
        match &self {
            BitfinexSubResponse::TradeSubscriptionMessage { .. } => Ok(self),
            BitfinexSubResponse::Error { msg, .. } => {
                Err(barter_integration::error::SocketError::Subscribe(format!(
                    "received failed subscription response: {}",
                    msg
                )))
            }
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
        sub_response: BitfinexSubResponse,
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
    HeartbeatEvent(i32, String),
    TradesSnapshotEvent(i32, Vec<BitfinexTrade>),
    TradesUpdateEvent(i32, String, BitfinexTrade),
    SubscriptionEvent(BitfinexSubResponse),
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
            IntermediaryBitfinexMessage::SubscriptionEvent(sub) => {
                match &sub {
                    BitfinexSubResponse::TradeSubscriptionMessage { 
                        event, 
                        channel, 
                        channel_id, 
                        symbol, 
                        pair 
                    } => {
                        let subscription_id = Bitfinex::subscription_id(&channel, &pair);
                        println!("SUBSCRIPTION ID: {}", subscription_id);
                        BitfinexMessage::SubscriptionEvent { 
                            subscription_id, 
                            sub_response: sub
                        }
                    },
                    BitfinexSubResponse::Error { event, msg, code, chan_id, symbol, channel } => {
                        // TODO: This is definitely wrong
                        let subscription_id = SubscriptionId::from("fwef");
                        BitfinexMessage::HeartbeatEvent { subscription_id, msg: "grs".to_string() }
                    },
                }
            },  
            IntermediaryBitfinexMessage::InfoEvent(info) => BitfinexMessage::InfoEvent { info },
        }
    }
}

/// The platform status (0 for maintenance, 1 for operative).
#[derive(Debug, Deserialize, Serialize, Copy, Clone)]
pub struct PlatformStatus {
    /// The status flag
    status: i8,
}

// TODO: Add enums for all info messages
/// Sent by the server to notify about the state of the connection.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum InfoMessage {
    /// Sent by server upon connection.
    SocketAcceptance {
        version: u8,
        server_id: String,
        #[serde(rename = "platform")]
        platform_status: PlatformStatus,
    },
}
