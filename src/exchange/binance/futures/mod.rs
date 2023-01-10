use self::{l2::BinanceFuturesBookUpdater, liquidation::BinanceLiquidation};
use super::{Binance, BinanceServer};
use crate::{
    exchange::{Connector, ExchangeId},
    subscription::{book::OrderBooksL2, liquidation::Liquidations},
    transformer::{book::multi::MultiBookTransformer, stateless::StatelessTransformer},
    ExchangeWsStream, StreamSelector,
};
use serde::{Deserialize, Serialize};

/// Todo:
pub mod l2;
pub mod liquidation;

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

impl StreamSelector<OrderBooksL2> for BinanceFuturesUsd {
    type Stream =
        ExchangeWsStream<MultiBookTransformer<Self, OrderBooksL2, BinanceFuturesBookUpdater>>;
}

impl StreamSelector<Liquidations> for BinanceFuturesUsd {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, Liquidations, BinanceLiquidation>>;
}

impl<'de> serde::Deserialize<'de> for BinanceFuturesUsd {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        match <String as serde::Deserialize>::deserialize(deserializer)?.as_str() {
            "BinanceFuturesUsd" | "binance_futures_usd" => Ok(Self::default()),
            other => Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(other),
                &"BinanceFuturesUsd | binance_futures_usd",
            )),
        }
    }
}

impl serde::Serialize for BinanceFuturesUsd {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(BinanceFuturesUsd::ID.as_str())
    }
}
