use barter_integration::{
    error::SocketError,
    model::{Exchange, Instrument},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Todo:
pub struct MarketIter<Event>(pub Vec<Result<Market<Event>, SocketError>>);

impl<Event> FromIterator<Result<Market<Event>, SocketError>> for MarketIter<Event> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Result<Market<Event>, SocketError>>,
    {
        Self(iter.into_iter().collect())
    }
}

/// Normalised Barter [`Market<Event>`](Self) containing metadata about the included `Event` variant.
///
/// Note: `Event` can be an enum if required.
///
/// eg/ Market<PublicTrade>
/// eg/ Market<OrderBook<Level3>>
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct Market<Event> {
    pub exchange_time: DateTime<Utc>,
    pub received_time: DateTime<Utc>,
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub event: Event,
}
