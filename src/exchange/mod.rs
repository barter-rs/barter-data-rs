

use barter_integration::model::Exchange;
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};


/// Todo:
pub mod binance;

// /// Ftx `ExchangeTransformer` & `Subscriber` implementations.
// pub mod ftx;
//
// /// Kraken `ExchangeTransformer` & `Subscriber` implementations.
// pub mod kraken;
//
// /// Coinbase `ExchangeTransformer` & `Subscriber` implementations.
// pub mod coinbase;

/// Todo: rust docs & check historical rust docs for inspo
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename = "exchange", rename_all = "snake_case")]
pub enum ExchangeId {
    BinanceFuturesUsd,
    BinanceSpot,
    Coinbase,
    Ftx,
    Kraken,
}

impl From<ExchangeId> for Exchange {
    fn from(exchange_id: ExchangeId) -> Self {
        Exchange::from(exchange_id.as_str())
    }
}

impl Display for ExchangeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl ExchangeId {
    /// Return the &str representation this [`ExchangeId`] is associated with.
    pub fn as_str(&self) -> &'static str {
        match self {
            ExchangeId::BinanceSpot => "binance",
            ExchangeId::BinanceFuturesUsd => "binance_futures_usd",
            ExchangeId::Coinbase => "coinbase",
            ExchangeId::Ftx => "ftx",
            ExchangeId::Kraken => "kraken",
        }
    }

    /// Todo: Find a way to delete all of this support non-sense
    /// Determines whether this [`ExchangeId`] supports the ingestion of
    /// [`InstrumentKind::Spot`](barter_integration::model::InstrumentKind) market data.
    #[allow(clippy::match_like_matches_macro)]
    pub fn supports_spot(&self) -> bool {
        match self {
            ExchangeId::BinanceFuturesUsd => false,
            _ => true,
        }
    }

    /// Determines whether this [`ExchangeId`] supports the collection of
    /// [`InstrumentKind::Future**`](barter_integration::model::InstrumentKind) market data.
    #[allow(clippy::match_like_matches_macro)]
    pub fn supports_futures(&self) -> bool {
        match self {
            ExchangeId::BinanceFuturesUsd => true,
            ExchangeId::Ftx => true,
            _ => false,
        }
    }

    /// Determines whether this [`ExchangeId`] supports the collection of
    /// [`PublicTrade`](model::PublicTrade) market data.
    #[allow(clippy::match_like_matches_macro)]
    #[allow(clippy::match_single_binding)]
    pub fn supports_trades(&self) -> bool {
        match self {
            _ => true,
        }
    }

    /// Determines whether this [`ExchangeId`] supports the collection of
    /// [`Candle`](model::Candle) market data.
    #[allow(clippy::match_like_matches_macro)]
    pub fn supports_candles(&self) -> bool {
        match self {
            ExchangeId::Kraken => true,
            _ => false,
        }
    }

    /// Determines whether this [`ExchangeId`] supports the collection of OrderBook snapshot
    /// market data.
    #[allow(clippy::match_like_matches_macro)]
    pub fn supports_order_books(&self) -> bool {
        match self {
            ExchangeId::BinanceFuturesUsd => true,
            _ => false,
        }
    }

    /// Determines whether this [`ExchangeId`] supports the collection of
    /// L2 OrderBook delta market data.
    #[allow(clippy::match_like_matches_macro)]
    #[allow(clippy::match_single_binding)]
    pub fn supports_order_book_l2_deltas(&self) -> bool {
        match self {
            _ => false,
        }
    }

    /// Determines whether this [`ExchangeId`] supports the collection of
    /// L3 OrderBook delta market data.
    #[allow(clippy::match_like_matches_macro)]
    #[allow(clippy::match_single_binding)]
    pub fn supports_order_book_l3_deltas(&self) -> bool {
        match self {
            _ => false,
        }
    }

    /// Determines whether this [`ExchangeId`] supports the collection of
    /// liquidation orders market data.
    #[allow(clippy::match_like_matches_macro)]
    pub fn supports_liquidations(&self) -> bool {
        match self {
            ExchangeId::BinanceFuturesUsd => true,
            _ => false,
        }
    }
}
