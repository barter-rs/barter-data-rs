use self::liquidation::BinanceLiquidation;
use super::{Binance, BinanceServer};
use crate::transformer::stateless::StatelessTransformer;
use crate::{
    exchange::ExchangeId,
    subscription::liquidation::Liquidations,
    ExchangeWsStream, StreamSelector,
};
use serde::{Deserialize, Serialize};

/// Todo:
pub mod liquidation;
pub mod l2;

/// [`BinanceFuturesUsd`] WebSocket server base url.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#websocket-market-streams>
pub const WEBSOCKET_BASE_URL_BINANCE_FUTURES_USD: &str = "wss://fstream.binance.com/ws";

/// [`BinanceFuturesUsd`] HTTP OrderBook snapshot url.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#order-book>
pub const HTTP_BOOK_SNAPSHOT_URL_BINANCE_SPOT: &str = "https://fapi.binance.com/fapi/v1/depth";

/// Todo:
///
pub type BinanceFuturesUsd = Binance<BinanceServerFuturesUsd>;

/// Todo:
///
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize,
)]
pub struct BinanceServerFuturesUsd;

impl BinanceServer for BinanceServerFuturesUsd {
    const ID: ExchangeId = ExchangeId::BinanceFuturesUsd;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_BINANCE_FUTURES_USD
    }

    fn http_book_snapshot_url() -> &'static str {
        HTTP_BOOK_SNAPSHOT_URL_BINANCE_SPOT
    }
}

impl StreamSelector<Liquidations> for BinanceFuturesUsd {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, Liquidations, BinanceLiquidation>>;
}
