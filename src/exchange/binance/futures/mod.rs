use self::liquidation::BinanceLiquidation;
use super::Binance;
use crate::{
    exchange::{ExchangeId, ServerSelector},
    ExchangeWsStream,
    StreamSelector, subscription::liquidation::Liquidations,
};
use serde::{Deserialize, Serialize};
use crate::transformer::stateless::StatelessTransformer;

/// Todo:
pub mod liquidation;

/// [`BinanceFuturesUsd`] server base url.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#websocket-market-streams>
pub const BASE_URL_BINANCE_FUTURES_USD: &str = "wss://fstream.binance.com/ws";

/// Todo:
///
pub type BinanceFuturesUsd = Binance<BinanceServerFuturesUsd>;

/// Todo:
///
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize,
)]
pub struct BinanceServerFuturesUsd;

impl ServerSelector for BinanceServerFuturesUsd {
    const ID: ExchangeId = ExchangeId::BinanceFuturesUsd;

    fn base_url() -> &'static str {
        BASE_URL_BINANCE_FUTURES_USD
    }
}

impl StreamSelector<Liquidations> for BinanceFuturesUsd {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, Liquidations, BinanceLiquidation>>;
}
