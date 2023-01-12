use barter_integration::{error::SocketError, Validator};
use serde::{Deserialize, Serialize};

/// [`Coinbase`](super::Coinbase) WebSocket subscription response.
///
/// ### Raw Payload Examples
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
/// #### Subscripion Success
/// ```json
/// {
///     "type":"subscriptions",
///     "channels":[
///         {"name":"matches","product_ids":["BTC-USD", "ETH-USD"]}
///     ]
/// }
/// ```
///
/// #### Subscription Failure
/// ```json
/// {
///     "type":"error",
///     "message":"Failed to subscribe",
///     "reason":"GIBBERISH-USD is not a valid product"
/// }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
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

/// Communicates the [`Coinbase`](super::Coinbase) product_ids (eg/ "ETH-USD") associated with
/// a successful channel (eg/ "matches") subscription.
///
/// See [`CoinbaseSubResponse`] for full raw paylaod examples.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    // Todo:
}
