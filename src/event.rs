use crate::error::DataError;
use barter_integration::{
    model::{Exchange, Instrument},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Todo:
pub struct MarketIter<Event>(pub Vec<Result<Market<Event>, DataError>>);

impl<Event> FromIterator<Result<Market<Event>, DataError>> for MarketIter<Event> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Result<Market<Event>, DataError>>,
    {
        Self(iter.into_iter().collect())
    }
}

/// Normalised Barter [`Market<Event>`](Self) wrapping the `Event` variant in metadata.
///
/// Note: `Event` can be an enum if required.
///
/// See [`crate::subscription`] for the existing `Event` variant data models.
///
/// eg/ [`Market<PublicTrade>`](crate::subscription::trade::PublicTrade)
/// eg/ [`Market<OrderBookLevel3>`](crate::subscription::book::OrderBookLevel3)
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct Market<Event> {
    pub exchange_time: DateTime<Utc>,
    pub received_time: DateTime<Utc>,
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub event: Event,
}
