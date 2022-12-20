use chrono::{DateTime, Utc};
use crate::subscriber::subscription::SubKind;

/// Barter [`Subscription`](super::Subscription) [`SubKind`] that yields [`OrderBook`]
/// [`Market`](crate::model::Market) events.
///
/// eg/ OrderBooks<Level3>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OrderBooks<Kind>;

impl<Kind> SubKind for OrderBooks<Kind> {
    type Event = OrderBook;
}

/// Level 1 [`OrderBook`] [`SubKind`].
///
/// Used in conjunction with the [`OrderBooks`] [`SubKind`] to yield non-aggregated, tick-by-tick,
/// top of book data.
pub struct Level1;

/// Level 2 [`OrderBook`] [`SubKind`].
///
/// Used in conjunction with the [`OrderBooks`] [`SubKind`] to yield [`OrderBook`]s aggregated
/// by price, tick-by-tick.
pub struct Level2;

/// Level 3 [`OrderBook`] [`SubKind`].
///
/// Used in conjunction with the [`OrderBooks`] [`SubKind`] to yield non-aggregated [`OrderBook`]s,
/// tick-by-tick - replication of the exchange [`OrderBook`].
pub struct Level3;

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