use serde::{Deserialize, Serialize};

/// Todo:
pub mod subscription;

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
