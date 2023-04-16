use self::{l2::BinanceFuturesBookUpdater, liquidation::BinanceLiquidation};
use super::{Binance, ExchangeServer};
use crate::{
    exchange::{ExchangeId, StreamSelector},
    subscription::{book::OrderBooksL2, liquidation::Liquidations},
    transformer::{book::MultiBookTransformer, stateless::StatelessTransformer},
    ExchangeWsStream,
};

/// Level 2 OrderBook types (top of book) and perpetual
/// [`OrderBookUpdater`](crate::transformer::book::OrderBookUpdater) implementation.
pub mod l2;

/// Liquidation types.
pub mod liquidation;

/// [`BinanceFuturesUsd`] WebSocket server base url.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#websocket-market-streams>
pub const WEBSOCKET_BASE_URL_BINANCE_FUTURES_USD: &str = "wss://fstream.binance.com/ws";

/// [`Binance`](super::Binance) perpetual usd exchange.
pub type BinanceFuturesUsd = Binance<BinanceServerFuturesUsd>;

/// [`Binance`](super::Binance) perpetual usd [`ExchangeServer`](super::super::ExchangeServer).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct BinanceServerFuturesUsd;

impl ExchangeServer for BinanceServerFuturesUsd {
    const ID: ExchangeId = ExchangeId::BinanceFuturesUsd;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_BINANCE_FUTURES_USD
    }
}

impl StreamSelector<OrderBooksL2> for BinanceFuturesUsd {
    type Stream =
        ExchangeWsStream<MultiBookTransformer<Self, OrderBooksL2, BinanceFuturesBookUpdater>>;
}

impl StreamSelector<Liquidations> for BinanceFuturesUsd {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, Liquidations, BinanceLiquidation>>;
}
