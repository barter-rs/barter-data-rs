use crate::error::DataError;
use barter_integration::model::{Exchange, Instrument};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Convenient new type containing a collection of [`MarketEvent<T>`](MarketEvent)s.
#[derive(Debug)]
pub struct MarketIter<T>(pub Vec<Result<MarketEvent<T>, DataError>>);

impl<T> FromIterator<Result<MarketEvent<T>, DataError>> for MarketIter<T> {
    fn from_iter<Iter>(iter: Iter) -> Self
    where
        Iter: IntoIterator<Item = Result<MarketEvent<T>, DataError>>,
    {
        Self(iter.into_iter().collect())
    }
}

/// Normalised Barter [`MarketEvent<T>`](Self) wrapping the `T` data variant in metadata.
///
/// Note: `T` can be an enum if required.
///
/// See [`crate::subscription`] for all existing Barter Market event variants.
///
/// ### Examples
/// - [`MarketEvent<PublicTrade>`](crate::subscription::trade::PublicTrade)
/// - [`MarketEvent<OrderBookL1>`](crate::subscription::book::OrderBookL1)
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct MarketEvent<T> {
    pub exchange_time: DateTime<Utc>,
    pub received_time: DateTime<Utc>,
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub kind: T,
}

/// Duplicates the standard library [`From`] trait. Enables the ergonomic improvements of
/// mapping a generic type into a generic type without conflicting with the std library
/// `impl<T> From<T> for T`.
///
/// Useful for the mapping a [`MarketEvent<X>`](MarketEvent) into a different [`MarketEvent<Y>`].
pub trait FromExt<Input> {
    fn from(value: Input) -> Self;
}

impl<Input, Output> FromExt<MarketEvent<Input>> for MarketEvent<Output>
where
    Output: From<Input>
{
    fn from(event: MarketEvent<Input>) -> Self {
        Self {
            exchange_time: event.exchange_time,
            received_time: event.received_time,
            exchange: event.exchange,
            instrument: event.instrument,
            kind: Output::from(event.kind)
        }
    }
}
