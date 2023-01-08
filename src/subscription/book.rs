use std::cmp::Ordering;
use super::SubKind;
use crate::{
    event::{MarketIter, Market},
    exchange::ExchangeId,
};
use barter_integration::{
    model::{Exchange, Instrument, Side},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

// Todo:
// - Remove un-required fields from OrderBookL1 & OrderBook (ie/ update fields)

/// Barter [`Subscription`](super::Subscription) [`SubKind`] that yields level 1 [`OrderBook`]
/// [`Market`](crate::model::Market) events.
///
/// Level 1 refers to the best non-aggregated bid and ask [`Level`] on each side of the
/// [`OrderBook`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OrderBooksL1;

impl SubKind for OrderBooksL1 {
    type Event = OrderBookL1;
}

/// Normalised Barter [`OrderBookL1`] snapshot containing the latest best bid and ask.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OrderBookL1 {
    pub last_update_time: DateTime<Utc>,
    pub last_update_id: u64,
    pub best_bid: Level,
    pub best_ask: Level,
}

/// Barter [`Subscription`](super::Subscription) [`SubKind`] that yields level 2 [`OrderBook`]
/// [`Market`](crate::model::Market) events.
///
/// Level 2 refers to the [`OrderBook`] aggregated by price.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OrderBooksL2;

impl SubKind for OrderBooksL2 {
    type Event = OrderBook;
}

/// Barter [`Subscription`](super::Subscription) [`SubKind`] that yields level 3 [`OrderBook`]
/// [`Market`](crate::model::Market) events.
///
/// Level 3 refers to the non-aggregated [`OrderBook`]. This is a direct replication of the exchange
/// [`OrderBook`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OrderBooksL3;

impl SubKind for OrderBooksL3 {
    type Event = OrderBook;
}

/// Normalised Barter [`OrderBook`] snapshot.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OrderBook {
    pub last_update_time: DateTime<Utc>,
    pub bids: OrderBookSide,
    pub asks: OrderBookSide,
}

impl OrderBook {
    /// Generate an [`OrderBook`] snapshot by cloning [`Self`] after sorting each [`OrderBookSide`].
    pub fn snapshot(&mut self) -> Self {
        // Sort OrderBook & Clone
        self.bids.sort();
        self.asks.sort();
        self.clone()
    }
}

/// Normalised Barter [`Level`]s for one [`Side`] of the [`OrderBook`].
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OrderBookSide {
    side: Side,
    levels: Vec<Level>,
}

impl OrderBookSide {
    /// Construct a new [`Self`] with the [`Level`]s provided.
    pub fn new<Iter, L>(side: Side, levels: Iter) -> Self
    where
        Iter: IntoIterator<Item = L>,
        L: Into<Level>,
    {
        Self {
            side,
            levels: levels
                .into_iter()
                .map(|level| level.into())
                .collect()
        }

    }

    /// Upsert a collection of [`Level`]s into this [`OrderBookSide`].
    pub fn upsert<Iter, L>(&mut self, levels: Iter)
    where
        Iter: IntoIterator<Item = L>,
        L: Into<Level>,
    {
        levels
            .into_iter()
            .for_each(|level| self.upsert_single(level))
    }

    /// Upsert a single [`Level`] into this [`OrderBookSide`].
    ///
    /// ### Upsert Scenarios
    /// #### 1 Level Already Exists
    /// 1a) New value is 0, remove the level
    /// 1b) New value is > 0, replace the level
    ///
    /// #### 2 Level Does Not Exist
    /// 2a) New value is > 0, insert new level
    /// 2b) New value is 0, log error and continue
    pub fn upsert_single<L>(&mut self, new_level: L)
    where
        L: Into<Level>,
    {
        let new_level = new_level.into();

        match self
            .levels
            .iter_mut()
            .enumerate()
            .find(|(_index, level)| level.eq_price(new_level.price))
        {
            // Scenario 1a: Level exists & new value is 0 => remove Level
            Some((index, _)) if new_level.price == 0.0 => {
                self.levels.remove(index);
            }

            // Scenario 1b: Level exists & new value is > 0 => replace Level
            Some((_, level)) => {
                *level = new_level;
            }

            // Scenario 2a: Level does not exist & new value > 0 => insert new Level
            None if new_level.price > 0.0 => {
                self.levels.push(new_level)
            }

            // Scenario 2b: Level does not exist & new value is 0 => log error & continue
            _ => {
                debug!(
                    ?new_level,
                    side = %self.side,
                    "Level to remove not found",
                );
            }
        };
    }

    /// Sort this [`OrderBookSide`] (bids are reversed).
    pub fn sort(&mut self) {
        // Sort Levels
        self.levels.sort_unstable();

        // Reverse Bids
        if let Side::Buy = self.side {
            self.levels.reverse();
        }
    }
}

/// Normalised Barter OrderBook [`Level`].
#[derive(Clone, Copy, PartialEq, Debug, Default, Deserialize, Serialize)]
pub struct Level {
    pub price: f64,
    pub amount: f64,
}

impl<T> From<(T, T)> for Level
where
    T: Into<f64>,
{
    fn from((price, amount): (T, T)) -> Self {
        Self::new(price, amount)
    }
}

impl Ord for Level {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other)
            .unwrap_or_else(|| panic!("{:?}.partial_cmp({:?}) impossible", self, other))
    }
}

impl PartialOrd for Level {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.price.partial_cmp(&other.price)? {
            Ordering::Equal => self.amount.partial_cmp(&other.amount),
            non_equal => Some(non_equal)
        }
    }
}

impl Eq for Level {}

impl Level {
    pub fn new<T>(price: T, amount: T) -> Self
    where
        T: Into<f64>,
    {
        Self {
            price: price.into(),
            amount: amount.into(),
        }
    }

    pub fn eq_price(&self, price: f64) -> bool {
        let diff = (price - self.price).abs();
        f64::EPSILON > diff
    }
}

impl From<(ExchangeId, Instrument, OrderBook)> for MarketIter<OrderBook> {
    fn from((exchange_id, instrument, book): (ExchangeId, Instrument, OrderBook)) -> Self {
        Self(vec![Ok(Market {
            exchange_time: book.last_update_time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            event: book,
        })])
    }
}
