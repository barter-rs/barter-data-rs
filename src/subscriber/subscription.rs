use crate::{
    Identifier,
    exchange::ExchangeId
};
use barter_integration::{
    model::SubscriptionId,
    protocol::websocket::WsMessage,
    error::SocketError,
    model::{Instrument, InstrumentKind, Symbol},
    Validator,
};
use std::{
    collections::HashMap,
    fmt::{Debug, Display, Formatter}
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use crate::model::{Liquidation, PublicTrade};

/// Todo:
pub trait SubscriptionIdentifier {
    fn subscription_id(&self) -> SubscriptionId;
}

/// Todo:
pub trait ExchangeMeta<ExchangeEvent>
where
    Self: Identifier<ExchangeId> + Clone,
    ExchangeEvent: SubscriptionIdentifier + for<'de> Deserialize<'de>,
{
    type ExchangeSub: ExchangeSubscription<ExchangeEvent>;

    fn base_url() -> &'static str;
}

/// Todo:
pub trait ExchangeSubscription<ExchangeEvent>
where
    Self: SubscriptionIdentifier + Sized,
    ExchangeEvent: SubscriptionIdentifier + for<'de> Deserialize<'de> // Todo: Do we need this?
{
    type SubResponse: Validator + DeserializeOwned;

    fn new<Kind>(subscription: &Subscription<Kind>) -> Self
    where
        Kind: SubKind;

    fn requests(subscriptions: Vec<Self>) -> Vec<WsMessage>;

    fn expected_responses(subscriptions: &Vec<WsMessage>) -> usize {
        subscriptions.len()
    }
}

/// Todo:
pub trait SubKind
where
    Self: Debug + Clone,
{
    type Event: Debug;
}

#[derive(Debug, Clone)]
pub struct PublicTrades;
impl SubKind for PublicTrades {
    type Event = PublicTrade;
}
#[derive(Debug, Clone)]
pub struct Liquidations;
impl SubKind for Liquidations {
    type Event = Liquidation;
}

/// Todo:
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Subscription<Kind> {
    pub exchange: ExchangeId,
    #[serde(flatten)]
    pub instrument: Instrument,
    #[serde(alias = "type")]
    pub kind: Kind,
}

impl<Kind> Display for Subscription<Kind>
where
    Kind: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{:?}{}", self.exchange, self.kind, self.instrument)
    }
}

impl<S, Kind> From<(ExchangeId, S, S, InstrumentKind, Kind)> for Subscription<Kind>
where
    S: Into<Symbol>,
{
    fn from((exchange, base, quote, instrument_kind, kind): (ExchangeId, S, S, InstrumentKind, Kind),
    ) -> Self {
        Self::new(exchange, (base, quote, instrument_kind), kind)
    }
}

impl<I, Kind> From<(ExchangeId, I, Kind)> for Subscription<Kind>
where
    I: Into<Instrument>,
{
    fn from((exchange, instrument, kind): (ExchangeId, I, Kind)) -> Self {
        Self::new(exchange, instrument, kind)
    }
}

// Todo: Check where this i sused and why I need funky trait bounds - cleaner way?
impl<Kind> From<Subscription<Kind> > for barter_integration::model::Market {
    fn from(subscription: Subscription<Kind> ) -> Self {
        Self::new(subscription.exchange, subscription.instrument)
    }
}

impl<Kind> Subscription<Kind>  {
    /// Constructs a new [`Subscription`] using the provided configuration.
    pub fn new<I>(exchange: ExchangeId, instrument: I, kind: Kind) -> Self
    where
        I: Into<Instrument>,
    {
        Self {
            exchange,
            instrument: instrument.into(),
            kind,
        }
    }
}

/// Todo: Check rust docs, etc.
/// Metadata generated from a collection of Barter [`Subscription`]s. This includes the exchange
/// specific subscription payloads that are sent to the exchange.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct SubscriptionMeta<Kind> {
    /// `HashMap` containing the mapping between an incoming exchange message's [`SubscriptionId`],
    /// and a Barter [`Subscription`]. Used to identify the original [`Subscription`] associated
    /// with a received message.
    pub map: SubscriptionMap<Kind>,
    /// Number of [`Subscription`] responses expected from the exchange. Used to validate all
    /// [`Subscription`] were accepted.
    pub expected_responses: usize,
    /// Collection of [`WsMessage`]s containing exchange specific subscription payloads to be sent.
    pub subscriptions: Vec<WsMessage>,
}

/// Todo: Use dyn SubKind?
/// Convenient type alias for a `HashMap` containing the mapping between an incoming exchange
/// message's [`SubscriptionId`], and a Barter [`Subscription`]. Used to identify the original
/// [`Subscription`] associated with a received message.
#[derive(Clone, Eq, PartialEq, Debug, Serialize)]
pub struct SubscriptionMap<Kind>(pub HashMap<SubscriptionId, Subscription<Kind>>);

impl<Kind> SubscriptionMap<Kind> {
    /// Find the [`Instrument`] associated with the provided [`SubscriptionId`] reference.
    pub fn find_instrument(&self, id: &SubscriptionId) -> Result<Instrument, SocketError> {
        self.0
            .get(id)
            .map(|subscription| subscription.instrument.clone())
            .ok_or_else(|| SocketError::Unidentifiable(id.clone()))
    }
}
