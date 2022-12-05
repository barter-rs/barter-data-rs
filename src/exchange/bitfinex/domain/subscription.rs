



/// [`Bitfinex`](super::Bitfinex) message variants received in response to WebSocket
/// subscription requests.
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
/// ### Subscription Error Response
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
    /// Success response to a connection request.
    Subscribed(BitfinexSubResponseKind),
    /// Error response to a connection request.
    Error(BitfinexError),
}