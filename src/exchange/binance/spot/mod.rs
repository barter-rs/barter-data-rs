use super::{Binance, BinanceServer};
use crate::exchange::ExchangeId;
use serde::{Deserialize, Serialize};

/// [`BinanceSpot`] WebSocket server base url.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#websocket-market-streams>
pub const WEBSOCKET_BASE_URL_BINANCE_SPOT: &str = "wss://stream.binance.com:9443/ws";

/// [`BinanceSpot`] HTTP OrderBook snapshot url.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#order-book>
pub const HTTP_BOOK_SNAPSHOT_URL_BINANCE_SPOT: &str = "https://api.binance.com/api/v3/depth";

/// Todo:
pub type BinanceSpot = Binance<BinanceServerSpot>;

/// Todo:
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize,
)]
pub struct BinanceServerSpot;

impl BinanceServer for BinanceServerSpot {
    const ID: ExchangeId = ExchangeId::BinanceSpot;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_BINANCE_SPOT
    }

    fn http_book_snapshot_url() -> &'static str {
        HTTP_BOOK_SNAPSHOT_URL_BINANCE_SPOT
    }
}
