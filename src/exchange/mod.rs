use crate::{
    Identifier,
    subscriber::subscription::{SubKind, Subscription, SubscriptionMap},
};
use barter_integration::{
    model::SubscriptionId,
    protocol::websocket::WsMessage,
    Validator,
};
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Todo:
pub mod coinbase;

/// Todo:
pub trait Connector {
    const ID: ExchangeId;
    type Channel;
    type Market;
    // type SubValidator: SubscriptionValidator;
    type SubResponse: Validator + DeserializeOwned;

    fn base_url() -> &'static str;

    fn subscription<Kind>(sub: &Subscription<Kind>) -> ExchangeSub<Self::Channel, Self::Market>
    where
        Kind: SubKind,
        Subscription<Kind>: Identifier<Self::Channel> + Identifier<Self::Market>,
{
    ExchangeSub { channel: sub.id(), market: sub.id() }
}

    fn subscription_id(sub: &ExchangeSub<Self::Channel, Self::Market>) -> SubscriptionId; // Todo: Perhaps I can hide this...?
    fn requests(subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage>;
    fn expected_responses<Kind>(map: &SubscriptionMap<Kind>) -> usize {
        map.0.len()
    }
}

pub struct ExchangeSub<Channel, Market> {
    channel: Channel,
    market: Market,
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

impl From<ExchangeId> for barter_integration::model::Exchange {
    fn from(exchange_id: ExchangeId) -> Self {
        barter_integration::model::Exchange::from(exchange_id.as_str())
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
