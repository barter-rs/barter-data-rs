use crate::subscriber::subscription::SubKind;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Barter [`Subscription`](super::Subscription) [`SubKind`] that yields level 1 [`OrderBook`]
/// [`Market`](crate::model::Market) events.
///
/// Level 1 refers to non-aggregated, tick-by-tick, top of [`OrderBook`] data.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OrderBooksL1;

impl SubKind for OrderBooksL1 {
    type Event = OrderBook;
}

/// Barter [`Subscription`](super::Subscription) [`SubKind`] that yields level 2 [`OrderBook`]
/// [`Market`](crate::model::Market) events.
///
/// Level 2 refers to aggregated by price, tick-by-tick, [`OrderBook`] data.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OrderBooksL2;

impl SubKind for OrderBooksL2 {
    type Event = OrderBook;
}

/// Barter [`Subscription`](super::Subscription) [`SubKind`] that yields level 3 [`OrderBook`]
/// [`Market`](crate::model::Market) events.
///
/// Level 3 refers to non-aggregated, tick-by-tick, [`OrderBook`] data. This is a direct
/// replication of the exchange [`OrderBook`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OrderBooksL3;

impl SubKind for OrderBooksL3 {
    type Event = OrderBook;
}

/// Normalised Barter [`OrderBook`] snapshot.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OrderBook {
    pub last_update_time: DateTime<Utc>,
    pub last_update_id: u64,
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
}

/// Normalised Barter [`OrderBook`] [`Level`].
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Level {
    pub price: f64,
    pub quantity: f64,
}

impl<T> From<(T, T)> for Level
where
    T: Into<f64>,
{
    fn from((price, quantity): (T, T)) -> Self {
        Self::new(price, quantity)
    }
}

impl Level {
    pub fn new<T>(price: T, quantity: T) -> Self
    where
        T: Into<f64>,
    {
        Self {
            price: price.into(),
            quantity: quantity.into(),
        }
    }
}
