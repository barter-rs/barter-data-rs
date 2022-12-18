use crate::{ExchangeWsStream, Identifier, MarketStream, subscriber::subscription::{Subscription, SubscriptionMap}};
use barter_integration::{model::SubscriptionId, protocol::websocket::WsMessage, Validator};
use std::fmt::{Debug, Display, Formatter};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use crate::exchange::coinbase::Coinbase;
use crate::exchange::coinbase::trade::CoinbaseTrade;
use crate::subscriber::Subscriber;
use crate::subscriber::subscription::SubKind;
use crate::subscriber::subscription::trade::PublicTrades;
use crate::subscriber::validator::SubscriptionValidator;
use crate::transformer::{ExchangeTransformer, StatelessTransformer};

/// Todo:
pub mod coinbase;

/// Todo: This is basically a subscriber... it has no connection to the Transformer
///  '--> do we like this... only connection is the ExchangeEvent I guess...?
///  '--> If the Connector could map SubKind -> ExchangeEvent that would SOLVE ALL OUR PROBLEMS
///      '--> Connector<SubKind> ? may want to revert back Transformer<Input> etc?
// pub trait Connector<Kind>
pub trait Connector
where
    Self: Clone + Sized,
    // Kind: SubKind, // Todo: Not sure we need this here, perhaps just on expected_responses()
{
    const ID: ExchangeId;

    type Channel: Debug;
    type Market: Debug;

    type Subscriber: Subscriber<Self::SubValidator>;
    type SubValidator: SubscriptionValidator;
    type SubResponse: Validator + DeserializeOwned;

    fn base_url() -> &'static str;
    fn requests(subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage>;
    fn expected_responses<Kind>(map: &SubscriptionMap<Self, Kind>) -> usize { map.0.len() }
}

pub trait StreamSelector<Kind>
where
    Self: Connector,
    Kind: SubKind,
{
    // Todo: This feels sub-optimal because Stream & Transformer are intimately connected...
    //       '--> Perhaps we should be de-coupling these at the b-integration::ExchangeStream level?
    type Stream: MarketStream<Self, Kind>;
    type Transformer: ExchangeTransformer<Self, Kind>;
    type Message: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>;
}

impl StreamSelector<PublicTrades> for Coinbase {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades>>;
    type Transformer = StatelessTransformer<Self, PublicTrades>;
    type Message = CoinbaseTrade;
}

/// Todo: rust docs & move somewhere...
pub struct ExchangeSub<Channel, Market> {
    channel: Channel,
    market: Market,
}

impl<Channel, Market> Identifier<SubscriptionId> for ExchangeSub<Channel, Market>
where
    Channel: Debug,
    Market: Debug,
{
    fn id(&self) -> SubscriptionId {
        SubscriptionId::from(format!("{:?}|{:?}", self.channel, self.market))
    }
}

impl<Channel, Market> ExchangeSub<Channel, Market> {
    pub fn new<Exchange, Kind>(sub: &Subscription<Exchange, Kind>) -> Self
    where
        Subscription<Exchange, Kind>: Identifier<Channel> + Identifier<Market>,
    {
        Self {
            channel: sub.id(),
            market: sub.id()
        }
    }
}

/// Todo: rust docs & check historical rust docs for inspiration
///   '--> Do we need this anymore? How can we assist users when they build there streams?
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
