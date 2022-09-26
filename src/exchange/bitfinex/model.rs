use crate::{
    exchange::{datetime_utc_from_epoch_duration, extract_next},
    model::{DataKind, MarketEvent, PublicTrade},
    ExchangeId,
};
use barter_integration::{
    error::SocketError,
    model::{Exchange, Instrument, Side},
    Validator,
};
use chrono::{DateTime, Utc};
use serde::{de::Error, Deserialize, Serialize};
use std::time::Duration;

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
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
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
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(tag = "channel", rename_all = "lowercase")]
pub enum BitfinexSubResponseKind {
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
/// [`SubscriptionId`](barter_integration::model::SubscriptionId).
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize)]
pub struct BitfinexMessage {
    pub channel_id: u32,
    pub payload: BitfinexPayload,
}

/// [`Bitfinex`](super::Bitfinex) market data variants associated with an
/// active [`Subscription`](crate::Subscription).
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize)]
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
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize)]
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

impl<'de> Deserialize<'de> for Status {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Outer {
            #[serde(deserialize_with = "de_status_from_u8")]
            status: Status,
        }

        // Deserialise Outer struct
        let Outer { status } = Outer::deserialize(deserializer)?;

        Ok(status)
    }
}

impl<'de> Deserialize<'de> for BitfinexMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct SeqVisitor;

        impl<'de> serde::de::Visitor<'de> for SeqVisitor {
            type Value = BitfinexMessage;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("BitfinexMessage struct from the Bitfinex WebSocket API")
            }

            fn visit_seq<SeqAccessor>(
                self,
                mut seq: SeqAccessor,
            ) -> Result<Self::Value, SeqAccessor::Error>
            where
                SeqAccessor: serde::de::SeqAccess<'de>,
            {
                // Trade: [CHANNEL_ID, <"te", "tu">, [ID, TIME, AMOUNT, PRICE]]
                // Heartbeat: [ CHANNEL_ID, "hb" ]

                // Extract CHANNEL_ID used to identify SubscriptionId: 1st element of the sequence
                let channel_id: u32 = extract_next(&mut seq, "channel_id")?;

                // Extract message tag used to identify type of payload: 2nd element of the sequence
                let message_tag: String = extract_next(&mut seq, "message_tag")?;

                // Use message tag to extract the payload: 3rd element of sequence
                let payload = match message_tag.as_str() {
                    // Filter "tu" Trades since they are identical but slower
                    // '--> use as additional Heartbeat
                    "hb" | "tu" => BitfinexPayload::Heartbeat,
                    "te" => BitfinexPayload::Trade(extract_next(&mut seq, "BitfinexTrade")?),
                    other => {
                        return Err(SeqAccessor::Error::unknown_variant(
                            other,
                            &["heartbeat (hb)", "trade (te | tu)"],
                        ))
                    }
                };

                // Ignore any additional elements or SerDe will fail
                //  '--> Bitfinex may add fields without warning
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}

                Ok(BitfinexMessage {
                    channel_id,
                    payload,
                })
            }
        }

        // Use Visitor implementation to deserialise the Bitfinex WebSocket message
        deserializer.deserialize_seq(SeqVisitor)
    }
}

impl<'de> Deserialize<'de> for BitfinexTrade {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct SeqVisitor;

        impl<'de> serde::de::Visitor<'de> for SeqVisitor {
            type Value = BitfinexTrade;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("BitfinexTrade struct from the Bitfinex WebSocket API")
            }

            fn visit_seq<SeqAccessor>(
                self,
                mut seq: SeqAccessor,
            ) -> Result<Self::Value, SeqAccessor::Error>
            where
                SeqAccessor: serde::de::SeqAccess<'de>,
            {
                // Trade: [ID, TIME, AMOUNT,PRICE]
                let id = extract_next(&mut seq, "id")?;
                let time_millis = extract_next(&mut seq, "time")?;
                let amount: f64 = extract_next(&mut seq, "amount")?;
                let price = extract_next(&mut seq, "price")?;
                let side = match amount.is_sign_positive() {
                    true => Side::Buy,
                    false => Side::Sell,
                };

                // Ignore any additional elements or SerDe will fail
                //  '--> Bitfinex may add fields without warning
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}

                Ok(BitfinexTrade {
                    id,
                    time: datetime_utc_from_epoch_duration(Duration::from_millis(time_millis)),
                    price,
                    quantity: amount.abs(),
                    side,
                })
            }
        }

        // Use Visitor implementation to deserialise the Bitfinex WebSocket message
        deserializer.deserialize_seq(SeqVisitor)
    }
}

/// Deserialize a `u8` as a `Bitfinex` platform [`Status`].
///
/// 0u8 => [`Status::Maintenance`](Status),
/// 1u8 => [`Status::Operative`](Status),
/// other => [`de::Error`]
fn de_status_from_u8<'de, D>(deserializer: D) -> Result<Status, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    match Deserialize::deserialize(deserializer)? {
        0 => Ok(Status::Maintenance),
        1 => Ok(Status::Operative),
        other => Err(D::Error::invalid_value(
            serde::de::Unexpected::Unsigned(other as u64),
            &"0 or 1",
        )),
    }
}
