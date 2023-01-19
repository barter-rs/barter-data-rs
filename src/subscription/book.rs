use super::SubKind;
use crate::{
    event::{MarketEvent, MarketIter},
    exchange::ExchangeId,
};
use barter_integration::model::{Exchange, Instrument, Side};
use barter_macro::{DeSubKind, SerSubKind};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use tracing::debug;

/// Barter [`Subscription`](super::Subscription) [`SubKind`] that yields level 1 [`OrderBook`]
/// [`MarketEvent<T>`](crate::event::MarketEvent) events.
///
/// Level 1 refers to the best non-aggregated bid and ask [`Level`] on each side of the
/// [`OrderBook`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, DeSubKind, SerSubKind)]
pub struct OrderBooksL1;

impl SubKind for OrderBooksL1 {
    type Event = OrderBookL1;
}

/// Normalised Barter [`OrderBookL1`] snapshot containing the latest best bid and ask.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OrderBookL1 {
    pub last_update_time: DateTime<Utc>,
    pub best_bid: Level,
    pub best_ask: Level,
}

/// Barter [`Subscription`](super::Subscription) [`SubKind`] that yields level 2 [`OrderBook`]
/// [`MarketEvent<T>`](crate::event::MarketEvent) events.
///
/// Level 2 refers to the [`OrderBook`] aggregated by price.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, DeSubKind, SerSubKind)]
pub struct OrderBooksL2;

impl SubKind for OrderBooksL2 {
    type Event = OrderBook;
}

/// Barter [`Subscription`](super::Subscription) [`SubKind`] that yields level 3 [`OrderBook`]
/// [`MarketEvent<T>`](crate::event::MarketEvent) events.
///
/// Level 3 refers to the non-aggregated [`OrderBook`]. This is a direct replication of the exchange
/// [`OrderBook`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, DeSubKind, SerSubKind)]
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
            levels: levels.into_iter().map(L::into).collect(),
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
            Some((index, _)) if new_level.amount == 0.0 => {
                self.levels.remove(index);
            }

            // Scenario 1b: Level exists & new value is > 0 => replace Level
            Some((_, level)) => {
                *level = new_level;
            }

            // Scenario 2a: Level does not exist & new value > 0 => insert new Level
            None if new_level.amount > 0.0 => self.levels.push(new_level),

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
            non_equal => Some(non_equal),
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
        Self(vec![Ok(MarketEvent {
            exchange_time: book.last_update_time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: book,
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod order_book_side {
        use super::*;

        #[test]
        fn test_upsert_single() {
            struct TestCase {
                book_side: OrderBookSide,
                new_level: Level,
                expected: OrderBookSide,
            }

            let tests = vec![
                TestCase {
                    // TC0: Level exists & new value is 0 => remove Level
                    book_side: OrderBookSide::new(
                        Side::Buy,
                        vec![Level::new(80, 1), Level::new(90, 1), Level::new(100, 1)],
                    ),
                    new_level: Level::new(100, 0),
                    expected: OrderBookSide::new(
                        Side::Buy,
                        vec![Level::new(80, 1), Level::new(90, 1)],
                    ),
                },
                TestCase {
                    // TC1: Level exists & new value is > 0 => replace Level
                    book_side: OrderBookSide::new(
                        Side::Buy,
                        vec![Level::new(80, 1), Level::new(90, 1), Level::new(100, 1)],
                    ),
                    new_level: Level::new(100, 10),
                    expected: OrderBookSide::new(
                        Side::Buy,
                        vec![Level::new(80, 1), Level::new(90, 1), Level::new(100, 10)],
                    ),
                },
                TestCase {
                    // TC2: Level does not exist & new value > 0 => insert new Level
                    book_side: OrderBookSide::new(
                        Side::Buy,
                        vec![Level::new(80, 1), Level::new(90, 1), Level::new(100, 1)],
                    ),
                    new_level: Level::new(110, 1),
                    expected: OrderBookSide::new(
                        Side::Buy,
                        vec![
                            Level::new(80, 1),
                            Level::new(90, 1),
                            Level::new(100, 1),
                            Level::new(110, 1),
                        ],
                    ),
                },
                TestCase {
                    // TC3: Level does not exist & new value is 0 => no change
                    book_side: OrderBookSide::new(
                        Side::Buy,
                        vec![Level::new(80, 1), Level::new(90, 1), Level::new(100, 1)],
                    ),
                    new_level: Level::new(110, 0),
                    expected: OrderBookSide::new(
                        Side::Buy,
                        vec![Level::new(80, 1), Level::new(90, 1), Level::new(100, 1)],
                    ),
                },
            ];

            for (index, mut test) in tests.into_iter().enumerate() {
                test.book_side.upsert_single(test.new_level);
                assert_eq!(test.book_side, test.expected, "TC{} failed", index);
            }
        }

        #[test]
        fn test_sort_bids() {
            struct TestCase {
                input: OrderBookSide,
                expected: OrderBookSide,
            }

            let tests = vec![
                TestCase {
                    // TC0: sorted correctly from reverse sorted
                    input: OrderBookSide::new(
                        Side::Buy,
                        vec![
                            Level::new(80, 1),
                            Level::new(90, 1),
                            Level::new(100, 1),
                            Level::new(110, 1),
                            Level::new(120, 1),
                        ],
                    ),
                    expected: OrderBookSide::new(
                        Side::Buy,
                        vec![
                            Level::new(120, 1),
                            Level::new(110, 1),
                            Level::new(100, 1),
                            Level::new(90, 1),
                            Level::new(80, 1),
                        ],
                    ),
                },
                TestCase {
                    // TC1: sorted correctly from partially sorted
                    input: OrderBookSide::new(
                        Side::Buy,
                        vec![
                            Level::new(120, 1),
                            Level::new(90, 1),
                            Level::new(80, 1),
                            Level::new(110, 1),
                            Level::new(100, 1),
                        ],
                    ),
                    expected: OrderBookSide::new(
                        Side::Buy,
                        vec![
                            Level::new(120, 1),
                            Level::new(110, 1),
                            Level::new(100, 1),
                            Level::new(90, 1),
                            Level::new(80, 1),
                        ],
                    ),
                },
                TestCase {
                    // TC1: sorted correctly from already sorted
                    input: OrderBookSide::new(
                        Side::Buy,
                        vec![
                            Level::new(120, 1),
                            Level::new(110, 1),
                            Level::new(100, 1),
                            Level::new(90, 1),
                            Level::new(80, 1),
                        ],
                    ),
                    expected: OrderBookSide::new(
                        Side::Buy,
                        vec![
                            Level::new(120, 1),
                            Level::new(110, 1),
                            Level::new(100, 1),
                            Level::new(90, 1),
                            Level::new(80, 1),
                        ],
                    ),
                },
            ];

            for (index, mut test) in tests.into_iter().enumerate() {
                test.input.sort();
                assert_eq!(test.input, test.expected, "TC{} failed", index);
            }
        }

        #[test]
        fn test_sort_asks() {
            struct TestCase {
                input: OrderBookSide,
                expected: OrderBookSide,
            }

            let tests = vec![
                TestCase {
                    // TC0: sorted correctly from already sorted
                    input: OrderBookSide::new(
                        Side::Sell,
                        vec![
                            Level::new(80, 1),
                            Level::new(90, 1),
                            Level::new(100, 1),
                            Level::new(110, 1),
                            Level::new(120, 1),
                        ],
                    ),
                    expected: OrderBookSide::new(
                        Side::Sell,
                        vec![
                            Level::new(80, 1),
                            Level::new(90, 1),
                            Level::new(100, 1),
                            Level::new(110, 1),
                            Level::new(120, 1),
                        ],
                    ),
                },
                TestCase {
                    // TC1: sorted correctly from partially sorted
                    input: OrderBookSide::new(
                        Side::Sell,
                        vec![
                            Level::new(120, 1),
                            Level::new(90, 1),
                            Level::new(80, 1),
                            Level::new(110, 1),
                            Level::new(100, 1),
                        ],
                    ),
                    expected: OrderBookSide::new(
                        Side::Sell,
                        vec![
                            Level::new(80, 1),
                            Level::new(90, 1),
                            Level::new(100, 1),
                            Level::new(110, 1),
                            Level::new(120, 1),
                        ],
                    ),
                },
                TestCase {
                    // TC1: sorted correctly from reverse sorted
                    input: OrderBookSide::new(
                        Side::Sell,
                        vec![
                            Level::new(120, 1),
                            Level::new(110, 1),
                            Level::new(100, 1),
                            Level::new(90, 1),
                            Level::new(80, 1),
                        ],
                    ),
                    expected: OrderBookSide::new(
                        Side::Sell,
                        vec![
                            Level::new(80, 1),
                            Level::new(90, 1),
                            Level::new(100, 1),
                            Level::new(110, 1),
                            Level::new(120, 1),
                        ],
                    ),
                },
            ];

            for (index, mut test) in tests.into_iter().enumerate() {
                test.input.sort();
                assert_eq!(test.input, test.expected, "TC{} failed", index);
            }
        }
    }

    mod level {
        use super::*;

        #[test]
        fn test_partial_ord() {
            struct TestCase {
                input_one: Level,
                input_two: Level,
                expected: Option<Ordering>,
            }

            let tests = vec![
                TestCase {
                    // TC0: Input One has higher price and higher quantity -> Greater
                    input_one: Level::new(100, 100),
                    input_two: Level::new(10, 10),
                    expected: Some(Ordering::Greater),
                },
                TestCase {
                    // TC1: Input One has higher price but same quantity -> Greater
                    input_one: Level::new(100, 100),
                    input_two: Level::new(10, 100),
                    expected: Some(Ordering::Greater),
                },
                TestCase {
                    // TC2: Input One has higher price but lower quantity -> Greater
                    input_one: Level::new(100, 10),
                    input_two: Level::new(10, 100),
                    expected: Some(Ordering::Greater),
                },
                TestCase {
                    // TC3: Input One has same price and higher quantity -> Greater
                    input_one: Level::new(10, 200),
                    input_two: Level::new(10, 100),
                    expected: Some(Ordering::Greater),
                },
                TestCase {
                    // TC4: Input One has same price and same quantity -> Equal
                    input_one: Level::new(100, 100),
                    input_two: Level::new(100, 100),
                    expected: Some(Ordering::Equal),
                },
                TestCase {
                    // TC5: Input One has same price but lower quantity -> Less
                    input_one: Level::new(10, 50),
                    input_two: Level::new(10, 100),
                    expected: Some(Ordering::Less),
                },
                TestCase {
                    // TC6: Input One has lower price but higher quantity -> Less
                    input_one: Level::new(10, 100),
                    input_two: Level::new(100, 50),
                    expected: Some(Ordering::Less),
                },
                TestCase {
                    // TC7: Input One has lower price and same quantity -> Less
                    input_one: Level::new(50, 100),
                    input_two: Level::new(100, 100),
                    expected: Some(Ordering::Less),
                },
                TestCase {
                    // TC8: Input One has lower price and lower quantity -> Less
                    input_one: Level::new(50, 50),
                    input_two: Level::new(100, 100),
                    expected: Some(Ordering::Less),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = test.input_one.partial_cmp(&test.input_two);
                match (actual, test.expected) {
                    (None, None) => {
                        // Test passed
                    }
                    (Some(actual), Some(expected)) => {
                        assert_eq!(actual, expected, "TC{} failed", index)
                    }
                    (actual, expected) => {
                        // Test failed
                        panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                    }
                }
            }
        }

        #[test]
        fn test_eq_price() {
            struct TestCase {
                level: Level,
                input_level: Level,
                expected: bool,
            }

            let tests = vec![
                TestCase {
                    // TC0: Input Level has higher price
                    level: Level::new(50, 100),
                    input_level: Level::new(100, 100),
                    expected: false,
                },
                TestCase {
                    // TC1: Input Level an equal price
                    level: Level::new(50, 100),
                    input_level: Level::new(50, 100),
                    expected: true,
                },
                TestCase {
                    // TC2: Input Level has lower price
                    level: Level::new(50, 100),
                    input_level: Level::new(10, 100),
                    expected: false,
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = test.level.eq_price(test.input_level.price);
                assert_eq!(actual, test.expected, "TC{} failed", index);
            }
        }
    }
}
