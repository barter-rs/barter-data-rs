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
/// See [`crate::subscription`] for all existing Barter Market event variants
/// (eg/ [`MarketEvent<PublicTrade>`](crate::subscription::trade::PublicTrade)).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct MarketEvent<T> {
    pub exchange_time: DateTime<Utc>,
    pub received_time: DateTime<Utc>,
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub event: T,
}
