use super::message::KrakenError;
use barter_integration::{
    error::SocketError,
    Validator,
};
use serde::{Deserialize, Serialize};

/// [`Kraken`] message received in response to WebSocket subscription requests.
///
/// ## Examples
/// ### Subscription Trade Ok Response
/// ```json
/// {
///   "channelID": 10001,
///   "channelName": "ticker",
///   "event": "subscriptionStatus",
///   "pair": "XBT/EUR",
///   "status": "subscribed",
///   "subscription": {
///     "name": "ticker"
///   }
/// }
/// ```
///
/// ### Subscription Trade Error Response
/// ```json
/// {
///   "errorMessage": "Subscription name invalid",
///   "event": "subscriptionStatus",
///   "pair": "XBT/USD",
///   "status": "error",
///   "subscription": {
///     "name": "trades"
///   }
/// }
/// ```
///
/// See docs: <https://docs.kraken.com/websockets/#message-subscriptionStatus>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum KrakenSubResponse {
    Subscribed {
        #[serde(alias = "channelID")]
        channel_id: u64,
        #[serde(alias = "channelName")]
        channel_name: String,
        pair: String,
    },
    Error(KrakenError),
}

impl Validator for KrakenSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self {
            KrakenSubResponse::Subscribed { .. } => Ok(self),
            KrakenSubResponse::Error(error) => Err(SocketError::Subscribe(format!(
                "received failure subscription response: {}",
                error.message
            ))),
        }
    }
}