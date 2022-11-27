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
    ops::{Deref, DerefMut},
    fmt::{Debug, Display, Formatter}
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Todo:
pub trait SubscriptionIdentifier {
    fn subscription_id(&self) -> SubscriptionId;
}

/// Todo:
pub trait ExchangeMeta<Event>
where
    Self: Identifier<ExchangeId>,
    Event: SubscriptionIdentifier + for<'de> Deserialize<'de>,
{
    type Sub: DomainSubscription<Event>;

    fn base_url() -> &'static str;
}

/// Todo:
pub trait SubKind
where
    Self: Debug + Clone,
{
    type Event: Debug;
}

/// Todo:
pub trait DomainSubscription<Event>
where
    Self: SubscriptionIdentifier + Sized,
    Event: SubscriptionIdentifier + DeserializeOwned
{
    type Response: Validator + DeserializeOwned;

    fn new<Kind: SubKind>(subscription: &Subscription<Kind>) -> Self;

    fn requests(subscriptions: Vec<Self>) -> Vec<WsMessage>;

    fn expected_responses(subscriptions: &Vec<WsMessage>) -> usize {
        subscriptions.len()
    }
}

/// Todo:
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Subscription<Exchange, Kind> {
    pub exchange: Exchange,
    #[serde(flatten)]
    pub instrument: Instrument,
    #[serde(alias = "type")]
    pub kind: Kind,
}

impl Validator for &Subscription {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        // Todo:
    }
}

impl<Exchange, Kind> Debug for Subscription<Exchange, Kind> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}{}", self.exchange, self.kind, self.instrument)
    }
}

impl Display for Subscription {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<S> From<(ExchangeId, S, S, InstrumentKind, SubKind)> for Subscription
where
    S: Into<Symbol>,
{
    fn from(
        (exchange, base, quote, instrument_kind, kind): (ExchangeId, S, S, InstrumentKind, SubKind),
    ) -> Self {
        Self::new(exchange, (base, quote, instrument_kind), kind)
    }
}

impl<I> From<(ExchangeId, I, SubKind)> for Subscription
where
    I: Into<Instrument>,
{
    fn from((exchange, instrument, stream): (ExchangeId, I, SubKind)) -> Self {
        Self::new(exchange, instrument, stream)
    }
}

impl From<Subscription> for barter_integration::model::Market {
    fn from(subscription: Subscription) -> Self {
        Self::new(subscription.exchange, subscription.instrument)
    }
}

impl Subscription {
    /// Constructs a new [`Subscription`] using the provided configuration.
    pub fn new<I>(exchange: ExchangeId, instrument: I, kind: SubKind) -> Self
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
pub struct SubscriptionMap<Kind: SubKind>(pub HashMap<SubscriptionId, Subscription<Kind>>);

impl<Kind> Deref for SubscriptionMap<Kind>
where
    Kind: SubKind,
{
    type Target = HashMap<SubscriptionId, Subscription<Kind>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<Kind> DerefMut for SubscriptionMap<Kind>
where
    Kind: SubKind,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<Kind> SubscriptionMap<Kind>
where
    Kind: SubKind,
{
    /// Find the [`Instrument`] associated with the provided [`SubscriptionId`] reference.
    pub fn find_instrument(&self, id: &SubscriptionId) -> Result<Instrument, SocketError> {
        self.get(id)
            .map(|subscription| subscription.instrument.clone())
            .ok_or_else(|| SocketError::Unidentifiable(id.clone()))
    }
}
