use crate::exchange::ExchangeId;
use super::{Bybit, ExchangeServer};

/// [`BybitFuturePerpetual`] WebSocket server base url.
///
/// See docs: <https://bybit-exchange.github.io/docs/v5/ws/connect>
pub const WEBSOCKET_BASE_URL_BYBIT_FUTURE_PERPETUAL: &str = "wss://stream.bybit.com/v5/public/linear";

/// [`Bybit`](super::Bybit) futures exchange.
pub type BybitFuturePerpetual = Bybit<BybitServerFuturePerpetual>;

/// [`Bybit`](super::Bybit) futures [`ExchangeServer`](super::super::ExchangeServer).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct BybitServerFuturePerpetual;

impl ExchangeServer for BybitServerFuturePerpetual {
    const ID: ExchangeId = ExchangeId::BybitFuturesUsd;

    fn websocket_url() -> &'static str { WEBSOCKET_BASE_URL_BYBIT_FUTURE_PERPETUAL }
}
