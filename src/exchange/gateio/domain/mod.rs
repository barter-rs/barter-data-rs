use barter_integration::{
    error::SocketError,
    Validator,
};
use serde::{Deserialize, Serialize};

/// Gateio WebSocket message.
///
/// Example: Subscription Ok Response
/// ```json
/// {
///   "time": 1606292218,
///   "time_ms": 1606292218231,
///   "channel": "spot.trades",
///   "event": "subscribe",
///   "result": {
///     "status": "success,
///     }
/// }
/// ```
/// Example: Trade
/// ```json
/// {
///   "time": 1606292218,
///   "time_ms": 1606292218231,
///   "channel": "spot.trades",
///   "event": "update",
///   "result": {
///     "id": 309143071,
///     "create_time": 1606292218,
///     "create_time_ms": "1606292218213.4578",
///     "side": "sell",
///     "currency_pair": "GT_USDT",
///     "amount": "16.4700000000",
///     "price": "0.4705000000"
///     }
/// }
/// ```
///
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#public-trades-channel>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct GateioMessage<T> {
    pub channel: String,
    pub error: Option<GateioError>,
    #[serde(rename = "result")]
    pub data: T,
}

/// Todo:
///
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#public-trades-channel>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct GateioError {
    pub code: u8,
    pub message: String,
}

/// Todo:
///
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#server-response>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct GateioSubResult {
    pub status: String,
}

impl Validator for GateioMessage<GateioSubResult> {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized
    {
        match &self.error {
            None => Ok(self),
            Some(failure) => Err(SocketError::Subscribe(format!(
                "received failure subscription response code: {} with message: {}",
                failure.code, failure.message,
            )))
        }
    }
}