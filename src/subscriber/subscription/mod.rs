use crate::{exchange::ExchangeId, Identifier};
use barter_integration::{
    error::SocketError,
    model::SubscriptionId,
    model::{Instrument, InstrumentKind, Symbol},
    protocol::websocket::WsMessage,
    Validator,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{Debug, Display, Formatter},
};

pub mod candle;
pub mod liquidation;
pub mod order_book;
/// Todo:
pub mod trade;

/// Todo:
pub trait ExchangeSubscription<ExchangeEvent>
where
    Self: Identifier<SubscriptionId> + Sized,
    ExchangeEvent: Identifier<SubscriptionId> + for<'de> Deserialize<'de>,
{
    type Channel;
    type SubResponse: Validator + DeserializeOwned;

    fn new<Kind>(sub: &Subscription<Kind>) -> Self
    where
        Kind: SubKind,
        Subscription<Kind>: Identifier<Self::Channel>;

    fn requests(subscriptions: Vec<Self>) -> Vec<WsMessage>;

    fn expected_responses<Kind>(subscription_map: &SubscriptionMap<Kind>) -> usize {
        subscription_map.0.len()
    }
}

/// Todo:
pub trait SubKind
where
    Self: Debug + Clone,
{
    type Event: Debug;
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
    fn from(
        (exchange, base, quote, instrument_kind, kind): (ExchangeId, S, S, InstrumentKind, Kind),
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
impl<Kind> From<Subscription<Kind>> for barter_integration::model::Market {
    fn from(subscription: Subscription<Kind>) -> Self {
        Self::new(subscription.exchange, subscription.instrument)
    }
}

impl<Kind> Subscription<Kind> {
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
    pub subscription_map: SubscriptionMap<Kind>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subscriber::subscription::trade::PublicTrades;

    #[test]
    fn test_subscription_map_find_instrument() {
        // Initialise SubscriptionIds HashMap
        let ids = SubscriptionMap(HashMap::from_iter([(
            SubscriptionId::from("present"),
            Subscription::from((
                ExchangeId::BinanceSpot,
                "base",
                "quote",
                InstrumentKind::Spot,
                PublicTrades,
            )),
        )]));

        struct TestCase {
            input: SubscriptionId,
            expected: Result<Instrument, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: SubscriptionId (channel) is present in the HashMap
                input: SubscriptionId::from("present"),
                expected: Ok(Instrument::from(("base", "quote", InstrumentKind::Spot))),
            },
            TestCase {
                // TC1: SubscriptionId (channel) is not present in the HashMap
                input: SubscriptionId::from("not present"),
                expected: Err(SocketError::Unidentifiable(SubscriptionId::from(
                    "not present",
                ))),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = ids.find_instrument(&test.input);
            match (actual, test.expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "TC{} failed", index)
                }
                (Err(_), Err(_)) => {
                    // Test passed
                }
                (actual, expected) => {
                    // Test failed
                    panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                }
            }
        }
    }
}
