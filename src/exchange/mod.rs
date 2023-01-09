use self::subscription::ExchangeSub;
use crate::{
    subscriber::{validator::SubscriptionValidator, Subscriber},
    subscription::InstrumentMap,
};
use barter_integration::{error::SocketError, protocol::websocket::WsMessage, Validator};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    time::Duration,
};
use url::Url;

/// Todo:
pub mod binance;
pub mod bitfinex;
pub mod coinbase;
pub mod gateio;
pub mod kraken;
pub mod okx;
pub mod subscription;

/// Default [`Duration`] the [`SubscriptionValidator`] will wait to receive all success responses
/// to actioned [`Subscription`] requests.
pub const DEFAULT_SUBSCRIPTION_TIMEOUT: Duration = Duration::from_secs(10);

/// Todo:
pub trait Connector
where
    Self: Clone + Debug + Sized,
{
    /// Unique identifier for the exchange server being connected with.
    const ID: ExchangeId;

    type Channel: AsRef<str>;
    type Market: AsRef<str>;
    type Subscriber: Subscriber<Self::SubValidator>;
    type SubValidator: SubscriptionValidator;
    type SubResponse: Validator + Debug + DeserializeOwned;

    /// Base Url of the exchange server to establish a connection with.
    fn url() -> Result<Url, SocketError>;

    fn ping_interval() -> Option<PingInterval> {
        None
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage>;

    /// Number of [`Subscription`] responses expected from the exchange in responses to the
    /// requests send. Used to validate all [`Subscription`]s were accepted.
    fn expected_responses(map: &InstrumentMap) -> usize {
        map.0.len()
    }

    /// Expected [`Duration`] the [`SubscriptionValidator`] will wait to receive all success
    /// responses to actioned [`Subscription`] requests.
    fn subscription_timeout() -> Duration {
        DEFAULT_SUBSCRIPTION_TIMEOUT
    }
}

/// Defines the frequency and recipe for custom [`WebSocket`] pings - used for exchanges that
/// require additional application-level pings.
#[derive(Debug)]
pub struct PingInterval {
    pub interval: tokio::time::Interval,
    pub ping: fn() -> WsMessage,
}

/// Todo:
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename = "exchange", rename_all = "snake_case")]
pub enum ExchangeId {
    BinanceFuturesUsd,
    BinanceSpot,
    Bitfinex,
    Coinbase,
    GateioFuturesBtc,
    GateioFuturesUsd,
    GateioSpot,
    Kraken,
    Okx,
}

impl From<ExchangeId> for barter_integration::model::Exchange {
    fn from(exchange_id: ExchangeId) -> Self {
        barter_integration::model::Exchange::from(exchange_id.as_str())
    }
}

impl Display for ExchangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

    /// Todo: Find a way to delete all of this support nonsense
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
