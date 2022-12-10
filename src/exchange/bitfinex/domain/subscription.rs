use crate::{
    model::{MarketIter, Market, PublicTrade},
    subscriber::subscription::{Subscription, trade::PublicTrades},
    ExchangeId, Identifier

};
use barter_integration::{
    error::SocketError,
    de::{datetime_utc_from_epoch_duration, extract_next},
    model::{Exchange, Instrument, Side, SubscriptionId},
    Validator,
};
use chrono::{DateTime, Utc};
use serde::{de::Error, Deserialize, Serialize};
use std::time::Duration;
use barter_integration::protocol::websocket::WsMessage;
use crate::subscriber::subscription::{ExchangeSubscription, SubKind};

/// Todo:
///
/// See docs: <https://docs.bitfinex.com/docs/ws-public>
#[derive(Debug, Copy, Clone)]
pub struct BitfinexChannel(pub &'static str);

impl BitfinexChannel {
    /// [`Bitfinex`] real-time trades channel.
    ///
    /// See docs: <https://docs.bitfinex.com/reference/ws-public-trades>
    pub const TRADES: Self = Self("trades");
}

impl Identifier<BitfinexChannel> for Subscription<PublicTrades> {
    fn id(&self) -> BitfinexChannel {
        BitfinexChannel::TRADES
    }
}

/// Todo:
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
pub struct BitfinexSubMeta {
    channel: BitfinexChannel,
    market: String,
}

impl Identifier<SubscriptionId> for BitfinexSubMeta {
    fn id(&self) -> SubscriptionId {
        subscription_id(self.channel, &self.market)
    }
}

impl<ExchangeEvent> ExchangeSubscription<ExchangeEvent> for BitfinexSubMeta {
    type Channel = BitfinexChannel;
    type SubResponse = BitfinexPlatformEvent;

    fn new<Kind>(sub: &Subscription<Kind>) -> Self 
    where 
        Kind: SubKind, 
        Subscription<Kind>: Identifier<Self::Channel> 
    {
        Self {
            channel: sub.id(),
            market: format!(
                "t{}{}",
                sub.instrument.base.to_string().to_uppercase(),
                sub.instrument.quote.to_string().to_uppercase()
            )
        }
    }

    fn requests(subscriptions: Vec<Self>) -> Vec<WsMessage> {
        todo!()
    }
}

/// Generate a [`Bitfinex`] [`SubscriptionId`] from the channel and market provided.
///
/// Uses "channel|tMARKET":
/// eg/ SubscriptionId("trades|tBTCUSD")
pub(crate) fn subscription_id(channel: BitfinexChannel, market: &str) -> SubscriptionId {
    panic!("remember the channel id replacement...");
    SubscriptionId::from(format!("{}|{}", channel.0, market))
}

/// Todo:
///
/// ### Platform Status Online
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
/// ### Subscription Trades Ok Response
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
/// ### Subscription Error Response
/// ``` json
/// {
///    "event": "error",
///    "msg": ERROR_MSG,
///    "code": ERROR_CODE
/// }
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum BitfinexPlatformEvent {
    /// [`Bitfinex`] platform status containing metadata about the server API.
    #[serde(rename = "info")]
    PlatformStatus(BitfinexPlatformStatus),
    /// Success response to a subscription request.
    Subscribed(BitfinexSubResponseKind),
    /// Error response to a subscription request.
    Error(BitfinexError),
}

impl Validator for BitfinexPlatformEvent {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized
    {
        match &self {
            BitfinexPlatformEvent::PlatformStatus(status) => match status.status {
                Status::Operative => Ok(self),
                Status::Maintenance => Err(SocketError::Subscribe(format!(
                    "exchange version: {} with server_id: {} is in maintenance mode",
                    status.api_version, status.server_id,
                ))),
            }
            BitfinexPlatformEvent::Subscribed(_) => Ok(self),
            BitfinexPlatformEvent::Error(error) => Err(SocketError::Subscribe(format!(
                "received failure subscription response code: {} with message: {}",
                error.code, error.msg,
            )))
        }
    }
}

/// [`Bitfinex`] platform status message containing the server we are connecting
/// to, the version of the API, and if it is in maintenance mode.
///
/// ## Examples
/// ### Platform Status Online
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
    #[serde(rename = "version")]
    api_version: u8,
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

/// [`Bitfinex`](super::Bitfinex) subscription success response variants for each channel.
///
/// ## Examples
/// ### Subscription Trades Ok Response
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
/// See docs: <https://docs.bitfinex.com/docs/ws-general#subscribe-to-channels>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(tag = "channel", rename_all = "lowercase")]
pub enum BitfinexSubResponse {
    /// Response to a connection request to the trade endpoint.
    Trades {
        /// The numeric `CHANNEL_ID` given to this stream.
        #[serde(rename = "chanId")]
        channel_id: u32,
        /// The symbol as formatted by Bitfinex, eg/ `tBTCUSD`
        #[serde(rename = "symbol")]
        market: String,
    },
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
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BitfinexError {
    msg: String,
    code: u32,
}
