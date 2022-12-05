use crate::{subscriber::subscription::ExchangeSubscription, ExchangeIdentifier, Identifier};
use barter_integration::model::{Exchange, SubscriptionId};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Todo:
pub mod binance;
pub mod coinbase;
pub mod gateio;
pub mod kraken;
pub mod okx;
pub mod bitfinex;

/// Todo:
pub trait ExchangeMeta<ExchangeEvent>
where
    Self: ExchangeIdentifier + Clone,
    ExchangeEvent: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
{
    type ExchangeSub: ExchangeSubscription<ExchangeEvent>;

    fn base_url() -> &'static str;
}

/// Todo: rust docs & check historical rust docs for inspiration
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename = "exchange", rename_all = "snake_case")]
pub enum ExchangeId {
    BinanceFuturesUsd,
    BinanceSpot,
    Bitfinex,
    Coinbase,
    GateioSpot,
    GateioFuturesUsd,
    GateioFuturesBtc,
    Kraken,
    Okx,
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
    /// Return the &str representation of this [`ExchangeId`]
    pub fn as_str(&self) -> &'static str {
        match self {
            ExchangeId::BinanceSpot => "binance_spot",
            ExchangeId::BinanceFuturesUsd => "binance_futures_usd",
            ExchangeId::Bitfinex => "bitfinex",
            ExchangeId::Coinbase => "coinbase",
            ExchangeId::GateioSpot => "gateio_spot",
            ExchangeId::GateioFuturesUsd => "gateio_futures_usd",
            ExchangeId::GateioFuturesBtc => "gateio_futures_btc",
            ExchangeId::Kraken => "kraken",
            ExchangeId::Okx => "okx",
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
            ExchangeId::Okx => true,
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
    /// liquidation orders market data.
    #[allow(clippy::match_like_matches_macro)]
    pub fn supports_liquidations(&self) -> bool {
        match self {
            ExchangeId::BinanceFuturesUsd => true,
            _ => false,
        }
    }
}
