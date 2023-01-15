use self::subscription::ExchangeSub;
use crate::{
    subscriber::{validator::SubscriptionValidator, Subscriber},
    subscription::Map,
};
use barter_integration::{
    error::SocketError, model::Instrument, protocol::websocket::WsMessage, Validator,
};
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
/// to actioned [`Subscription`](crate::subscription::Subscription) requests.
pub const DEFAULT_SUBSCRIPTION_TIMEOUT: Duration = Duration::from_secs(10);

/// Todo:
pub trait Connector
where
    Self: Clone + Default + Debug + for<'de> Deserialize<'de> + Serialize + Sized,
{
    /// Unique identifier for the exchange server being connected with.
    const ID: ExchangeId;

    /// Type that defines how to translate a Barter [`Subscription`] into an exchange specific
    /// channel to be subscribed to.
    ///
    /// ### Examples
    /// - [`BinanceChannel("@depth@100ms")`](BinanceChannel)
    /// - [`KrakenChannel("trade")`](KrakenChannel)
    type Channel: AsRef<str>;

    /// Type that defines how to translate a Barter [`Subscription`] into an exchange specific
    /// market that can be subscribed to.
    ///
    /// ### Examples
    /// - [`BinanceMarket("btcusdt")`](BinanceMarket)
    /// - [`KrakenMarket("BTC/USDT")`](KrakenMarket)
    type Market: AsRef<str>;

    /// Todo:
    type Subscriber: Subscriber;


    type SubValidator: SubscriptionValidator;

    /// Deserialisable type that the [`Self::SubValidator`] expects to receive from the exchange server in
    /// response to the [`Subscription`] [`Self::requests`] sent over the [`WebSocket`].
    /// Implements [`Validator`] in order to determine if [`Self`] communicates a successful
    /// [`Subscription`] outcome.
    type SubResponse: Validator + Debug + DeserializeOwned;

    /// Base [`Url`] of the exchange server being connected with.
    fn url() -> Result<Url, SocketError>;

    /// Defines [`PingInterval`] of custom application-level
    /// [`WebSocket`](barter_integration::protocol::websocket::WebSocket) pings for the exchange
    /// server being connected with.
    ///
    /// Defaults to `None`, meaning that no custom pings are sent.
    fn ping_interval() -> Option<PingInterval> {
        None
    }

    /// Defines how to translate a collection of [`ExchangeSub`]s into the [`WsMessage`]
    /// subscription payloads sent to the exchange server.
    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage>;

    /// Number of [`Subscription`](crate::subscription::Subscription) responses expected from the
    /// exchange server in responses to the requests send. Used to validate all
    /// [`Subscription`](crate::subscription::Subscription)s were accepted.
    fn expected_responses(map: &Map<Instrument>) -> usize {
        map.0.len()
    }

    /// Expected [`Duration`] the [`SubscriptionValidator`] will wait to receive all success
    /// responses to actioned [`Subscription`](crate::subscription::Subscription) requests.
    fn subscription_timeout() -> Duration {
        DEFAULT_SUBSCRIPTION_TIMEOUT
    }
}

/// Defines the frequency and construction function for custom
/// [`WebSocket`](barter_integration::protocol::websocket::WebSocket) pings - used for exchanges
/// that require additional application-level pings.
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
}

/// Todo:
pub trait ExchangeServer: Default + Debug + Clone + Send {
    const ID: ExchangeId;
    fn websocket_url() -> &'static str;
}
