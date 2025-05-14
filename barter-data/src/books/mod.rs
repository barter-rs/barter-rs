use crate::subscription::book::OrderBookEvent;
use chrono::{DateTime, Utc};
use derive_more::Display;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use tracing::debug;

/// Provides a [`OrderBookL2Manager`](manager::OrderBookL2Manager) for maintaining a set of local
/// L2 [`OrderBook`]s.
pub mod manager;

/// Provides an abstract collection of cheaply cloneable shared-state [`OrderBook`].
pub mod map;

/// Normalised Barter [`OrderBook`] snapshot.
#[derive(Clone, PartialEq, Eq, Debug, Default, Deserialize, Serialize)]
pub struct OrderBook {
    sequence: u64,
    time_engine: Option<DateTime<Utc>>,
    bids: OrderBookSide<Bids>,
    asks: OrderBookSide<Asks>,
}

impl OrderBook {
    /// Construct a new sorted [`OrderBook`].
    ///
    /// Note that the passed bid and asks levels do not need to be pre-sorted.
    pub fn new<IterBids, IterAsks, L>(
        sequence: u64,
        time_engine: Option<DateTime<Utc>>,
        bids: IterBids,
        asks: IterAsks,
    ) -> Self
    where
        IterBids: IntoIterator<Item = L>,
        IterAsks: IntoIterator<Item = L>,
        L: Into<Level>,
    {
        Self {
            sequence,
            time_engine,
            bids: OrderBookSide::bids(bids),
            asks: OrderBookSide::asks(asks),
        }
    }

    /// Current `u64` sequence number associated with the [`OrderBook`].
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Current engine time associated with the [`OrderBook`].
    pub fn time_engine(&self) -> Option<DateTime<Utc>> {
        self.time_engine
    }

    /// Generate a sorted [`OrderBook`] snapshot with a maximum depth.
    pub fn snapshot(&self, depth: usize) -> Self {
        Self {
            sequence: self.sequence,
            time_engine: self.time_engine,
            bids: OrderBookSide::bids(self.bids.levels.iter().take(depth).copied()),
            asks: OrderBookSide::asks(self.asks.levels.iter().take(depth).copied()),
        }
    }

    /// Update the local [`OrderBook`] from a new [`OrderBookEvent`].
    pub fn update(&mut self, event: &OrderBookEvent) {
        match event {
            OrderBookEvent::Snapshot(snapshot) => {
                *self = snapshot.clone();
            }
            OrderBookEvent::Update(update) => {
                self.sequence = update.sequence;
                self.time_engine = update.time_engine;
                self.upsert_bids(&update.bids);
                self.upsert_asks(&update.asks);
            }
        }
    }

    /// Update the local [`OrderBook`] by upserting the levels in an [`OrderBookSide`].
    fn upsert_bids(&mut self, update: &OrderBookSide<Bids>) {
        self.bids.upsert(&update.levels)
    }

    /// Update the local [`OrderBook`] by upserting the levels in an [`OrderBookSide`].
    fn upsert_asks(&mut self, update: &OrderBookSide<Asks>) {
        self.asks.upsert(&update.levels)
    }

    /// Return a reference to this [`OrderBook`]s bids.
    pub fn bids(&self) -> &OrderBookSide<Bids> {
        &self.bids
    }

    /// Return a reference to this [`OrderBook`]s asks.
    pub fn asks(&self) -> &OrderBookSide<Asks> {
        &self.asks
    }

    /// Calculate the mid-price by taking the average of the best bid and ask prices.
    ///
    /// See Docs: <https://www.quantstart.com/articles/high-frequency-trading-ii-limit-order-book>
    pub fn mid_price(&self) -> Option<Decimal> {
        match (self.bids.best(), self.asks.best()) {
            (Some(best_bid), Some(best_ask)) => Some(mid_price(best_bid.price, best_ask.price)),
            (Some(best_bid), None) => Some(best_bid.price),
            (None, Some(best_ask)) => Some(best_ask.price),
            (None, None) => None,
        }
    }

    /// Calculate the volume weighted mid-price (micro-price), weighing the best bid and ask prices
    /// with their associated amount.
    ///
    /// See Docs: <https://www.quantstart.com/articles/high-frequency-trading-ii-limit-order-book>
    pub fn volume_weighed_mid_price(&self) -> Option<Decimal> {
        match (self.bids.best(), self.asks.best()) {
            (Some(best_bid), Some(best_ask)) => {
                Some(volume_weighted_mid_price(*best_bid, *best_ask))
            }
            (Some(best_bid), None) => Some(best_bid.price),
            (None, Some(best_ask)) => Some(best_ask.price),
            (None, None) => None,
        }
    }
}

/// Normalised Barter [`Level`]s for one `Side` of the [`OrderBook`].
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct OrderBookSide<Side> {
    #[serde(skip_serializing)]
    pub side: Side,
    levels: Vec<Level>,
}

/// Unit type to tag an [`OrderBookSide`] as the bid Side (ie/ buyers) of an [`OrderBook`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Display)]
pub struct Bids;

/// Unit type to tag an [`OrderBookSide`] as the ask Side (ie/ sellers) of an [`OrderBook`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Display)]
pub struct Asks;

impl OrderBookSide<Bids> {
    /// Construct a new [`OrderBookSide<Bids>`] from the provided [`Level`]s.
    pub fn bids<Iter, L>(levels: Iter) -> Self
    where
        Iter: IntoIterator<Item = L>,
        L: Into<Level>,
    {
        let mut levels = levels.into_iter().map(L::into).collect::<Vec<_>>();
        levels.sort_unstable_by(|a, b| a.price.cmp(&b.price).reverse());

        Self { side: Bids, levels }
    }

    /// Upsert bid [`Level`]s into this [`OrderBookSide<Bids>`].
    pub fn upsert<L>(&mut self, levels: &[L])
    where
        L: Into<Level> + Copy,
    {
        levels.iter().for_each(|upsert| {
            let upsert: Level = (*upsert).into();
            self.upsert_single(upsert, |existing| {
                existing.price.cmp(&upsert.price).reverse()
            })
        })
    }
}

impl OrderBookSide<Asks> {
    /// Construct a new [`OrderBookSide<Asks>`] from the provided [`Level`]s.
    pub fn asks<Iter, L>(levels: Iter) -> Self
    where
        Iter: IntoIterator<Item = L>,
        L: Into<Level>,
    {
        let mut levels = levels.into_iter().map(L::into).collect::<Vec<_>>();
        levels.sort_unstable_by(|a, b| a.price.cmp(&b.price));

        Self { side: Asks, levels }
    }

    /// Upsert ask [`Level`]s into this [`OrderBookSide<Asks>`].
    pub fn upsert<L>(&mut self, levels: &[L])
    where
        L: Into<Level> + Copy,
    {
        levels.into_iter().for_each(|upsert| {
            let upsert = (*upsert).into();
            self.upsert_single(upsert, |existing| existing.price.cmp(&upsert.price))
        })
    }
}

impl<Side> OrderBookSide<Side>
where
    Side: std::fmt::Display + std::fmt::Debug,
{
    /// Get best [`Level`] on the [`OrderBookSide`].
    pub fn best(&self) -> Option<&Level> {
        self.levels.first()
    }

    /// Return a reference to the [`OrderBookSide`] levels.
    pub fn levels(&self) -> &[Level] {
        &self.levels
    }

    /// Upsert a single [`Level`] into this [`OrderBookSide`].
    ///
    /// ### Upsert Scenarios
    /// #### 1 Level Already Exists
    /// 1a) New value is 0, remove the level
    /// 1b) New value is > 0, replace the level
    ///
    /// #### 2 Level Does Not Exist
    /// 2a) New value is 0, log warn and continue
    /// 2b) New value is > 0, insert new level
    pub fn upsert_single<FnOrd>(&mut self, new_level: Level, fn_ord: FnOrd)
    where
        FnOrd: Fn(&Level) -> Ordering,
    {
        match (self.levels.binary_search_by(fn_ord), new_level.amount) {
            (Ok(index), new_amount) => {
                if new_amount.is_zero() {
                    // Scenario 1a: Level exists & new value is 0 => remove level
                    let _removed = self.levels.remove(index);
                } else {
                    // Scenario 1b: Level exists & new value is > 0 => replace level
                    self.levels[index].amount = new_amount;
                }
            }
            (Err(index), new_amount) => {
                if new_amount.is_zero() {
                    // Scenario 2a: Level does not exist & new value is 0 => log & continue
                    debug!(
                        ?new_level,
                        side = %self.side,
                        "received upsert Level with zero amount (to remove) that was not found"
                    );
                } else {
                    // Scenario 2b: Level does not exist & new value > 0 => insert new level
                    self.levels.insert(index, new_level);
                }
            }
        }
    }
}

impl Default for OrderBookSide<Bids> {
    fn default() -> Self {
        Self {
            side: Bids,
            levels: vec![],
        }
    }
}

impl Default for OrderBookSide<Asks> {
    fn default() -> Self {
        Self {
            side: Asks,
            levels: vec![],
        }
    }
}

/// Normalised Barter OrderBook [`Level`].
#[derive(Debug, Copy, Clone, PartialEq, Ord, PartialOrd, Hash, Default, Deserialize, Serialize)]
pub struct Level {
    pub price: Decimal,
    pub amount: Decimal,
}

impl<T> From<(T, T)> for Level
where
    T: Into<Decimal>,
{
    fn from((price, amount): (T, T)) -> Self {
        Self::new(price, amount)
    }
}

impl Eq for Level {}

impl Level {
    pub fn new<T>(price: T, amount: T) -> Self
    where
        T: Into<Decimal>,
    {
        Self {
            price: price.into(),
            amount: amount.into(),
        }
    }
}

/// Calculate the mid-price by taking the average of the best bid and ask prices.
///
/// See Docs: <https://www.quantstart.com/articles/high-frequency-trading-ii-limit-order-book>
pub fn mid_price(best_bid_price: Decimal, best_ask_price: Decimal) -> Decimal {
    (best_bid_price + best_ask_price) / Decimal::TWO
}

/// Calculate the volume weighted mid-price (micro-price), weighing the best bid and ask prices
/// with their associated amount.
///
/// See Docs: <https://www.quantstart.com/articles/high-frequency-trading-ii-limit-order-book>
pub fn volume_weighted_mid_price(best_bid: Level, best_ask: Level) -> Decimal {
    ((best_bid.price * best_ask.amount) + (best_ask.price * best_bid.amount))
        / (best_bid.amount + best_ask.amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod order_book_l1 {
        use super::*;
        use crate::subscription::book::OrderBookL1;
        use rust_decimal_macros::dec;

        #[test]
        fn test_mid_price() {
            struct TestCase {
                input: OrderBookL1,
                expected: Option<Decimal>,
            }

            let tests = vec![
                TestCase {
                    // TC0
                    input: OrderBookL1 {
                        last_update_time: Default::default(),
                        best_bid: Some(Level::new(100, 999999)),
                        best_ask: Some(Level::new(200, 1)),
                    },
                    expected: Some(dec!(150.0)),
                },
                TestCase {
                    // TC1
                    input: OrderBookL1 {
                        last_update_time: Default::default(),
                        best_bid: Some(Level::new(50, 1)),
                        best_ask: Some(Level::new(250, 999999)),
                    },
                    expected: Some(dec!(150.0)),
                },
                TestCase {
                    // TC2
                    input: OrderBookL1 {
                        last_update_time: Default::default(),
                        best_bid: Some(Level::new(10, 999999)),
                        best_ask: Some(Level::new(250, 999999)),
                    },
                    expected: Some(dec!(130.0)),
                },
                TestCase {
                    // TC3
                    input: OrderBookL1 {
                        last_update_time: Default::default(),
                        best_bid: Some(Level::new(10, 999999)),
                        best_ask: None,
                    },
                    expected: None,
                },
                TestCase {
                    // TC4
                    input: OrderBookL1 {
                        last_update_time: Default::default(),
                        best_bid: None,
                        best_ask: Some(Level::new(250, 999999)),
                    },
                    expected: None,
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                assert_eq!(test.input.mid_price(), test.expected, "TC{index} failed")
            }
        }

        #[test]
        fn test_volume_weighted_mid_price() {
            struct TestCase {
                input: OrderBookL1,
                expected: Option<Decimal>,
            }

            let tests = vec![
                TestCase {
                    // TC0: volume the same so should be equal to non-weighted mid price
                    input: OrderBookL1 {
                        last_update_time: Default::default(),
                        best_bid: Some(Level::new(100, 100)),
                        best_ask: Some(Level::new(200, 100)),
                    },
                    expected: Some(dec!(150.0)),
                },
                TestCase {
                    // TC1: volume affects mid-price
                    input: OrderBookL1 {
                        last_update_time: Default::default(),
                        best_bid: Some(Level::new(100, 600)),
                        best_ask: Some(Level::new(200, 1000)),
                    },
                    expected: Some(dec!(137.5)),
                },
                TestCase {
                    // TC2: volume the same and price the same
                    input: OrderBookL1 {
                        last_update_time: Default::default(),
                        best_bid: Some(Level::new(1000, 999999)),
                        best_ask: Some(Level::new(1000, 999999)),
                    },
                    expected: Some(dec!(1000.0)),
                },
                TestCase {
                    // TC3: best ask is None
                    input: OrderBookL1 {
                        last_update_time: Default::default(),
                        best_bid: Some(Level::new(1000, 999999)),
                        best_ask: None,
                    },
                    expected: None,
                },
                TestCase {
                    // TC4: best bid is None
                    input: OrderBookL1 {
                        last_update_time: Default::default(),
                        best_bid: None,
                        best_ask: Some(Level::new(1000, 999999)),
                    },
                    expected: None,
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                assert_eq!(
                    test.input.volume_weighed_mid_price(),
                    test.expected,
                    "TC{index} failed"
                )
            }
        }
    }

    mod order_book {
        use super::*;
        use rust_decimal_macros::dec;

        #[test]
        fn test_mid_price() {
            struct TestCase {
                input: OrderBook,
                expected: Option<Decimal>,
            }

            let tests = vec![
                TestCase {
                    // TC0: no levels so 0.0 mid-price
                    input: OrderBook::new::<Vec<_>, Vec<_>, Level>(
                        0,
                        Default::default(),
                        vec![],
                        vec![],
                    ),
                    expected: None,
                },
                TestCase {
                    // TC1: no asks in the books so take best bid price
                    input: OrderBook::new(
                        0,
                        Default::default(),
                        vec![
                            Level::new(dec!(100.0), dec!(100.0)),
                            Level::new(dec!(50.0), dec!(100.0)),
                        ],
                        vec![],
                    ),
                    expected: Some(dec!(100.0)),
                },
                TestCase {
                    // TC2: no bids in the books so take ask price
                    input: OrderBook::new(
                        0,
                        Default::default(),
                        vec![],
                        vec![
                            Level::new(dec!(50.0), dec!(100.0)),
                            Level::new(dec!(100.0), dec!(100.0)),
                        ],
                    ),
                    expected: Some(dec!(50.0)),
                },
                TestCase {
                    // TC3: best bid and ask amount is the same, so regular mid-price
                    input: OrderBook::new(
                        0,
                        Default::default(),
                        vec![
                            Level::new(dec!(100.0), dec!(100.0)),
                            Level::new(dec!(50.0), dec!(100.0)),
                        ],
                        vec![
                            Level::new(dec!(200.0), dec!(100.0)),
                            Level::new(dec!(300.0), dec!(100.0)),
                        ],
                    ),
                    expected: Some(dec!(150.0)),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                assert_eq!(test.input.mid_price(), test.expected, "TC{index} failed")
            }
        }

        #[test]
        fn test_volume_weighted_mid_price() {
            struct TestCase {
                input: OrderBook,
                expected: Option<Decimal>,
            }

            let tests = vec![
                TestCase {
                    // TC0: no levels so 0.0 mid-price
                    input: OrderBook::new::<Vec<_>, Vec<_>, Level>(
                        0,
                        Default::default(),
                        vec![],
                        vec![],
                    ),
                    expected: None,
                },
                TestCase {
                    // TC1: no asks in the books so take best bid price
                    input: OrderBook::new(
                        0,
                        Default::default(),
                        vec![
                            Level::new(dec!(100.0), dec!(100.0)),
                            Level::new(dec!(50.0), dec!(100.0)),
                        ],
                        vec![],
                    ),
                    expected: Some(dec!(100.0)),
                },
                TestCase {
                    // TC2: no bids in the books so take ask price
                    input: OrderBook::new(
                        0,
                        Default::default(),
                        vec![],
                        vec![
                            Level::new(dec!(50.0), dec!(100.0)),
                            Level::new(dec!(100.0), dec!(100.0)),
                        ],
                    ),
                    expected: Some(dec!(50.0)),
                },
                TestCase {
                    // TC3: best bid and ask amount is the same, so regular mid-price
                    input: OrderBook::new(
                        0,
                        Default::default(),
                        vec![
                            Level::new(dec!(100.0), dec!(100.0)),
                            Level::new(dec!(50.0), dec!(100.0)),
                        ],
                        vec![
                            Level::new(dec!(200.0), dec!(100.0)),
                            Level::new(dec!(300.0), dec!(100.0)),
                        ],
                    ),
                    expected: Some(dec!(150.0)),
                },
                TestCase {
                    // TC4: valid volume weighted mid-price
                    input: OrderBook::new(
                        0,
                        Default::default(),
                        vec![
                            Level::new(dec!(100.0), dec!(3000.0)),
                            Level::new(dec!(50.0), dec!(100.0)),
                        ],
                        vec![
                            Level::new(dec!(200.0), dec!(1000.0)),
                            Level::new(dec!(300.0), dec!(100.0)),
                        ],
                    ),
                    expected: Some(dec!(175.0)),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                assert_eq!(
                    test.input.volume_weighed_mid_price(),
                    test.expected,
                    "TC{index} failed"
                )
            }
        }
    }

    mod order_book_side {
        use super::*;
        use rust_decimal_macros::dec;

        #[test]
        fn test_upsert_single() {
            struct TestCase {
                book_side: OrderBookSide<Bids>,
                new_level: Level,
                expected: OrderBookSide<Bids>,
            }

            let tests = vec![
                TestCase {
                    // TC0: Level exists & new value is 0 => remove Level
                    book_side: OrderBookSide::bids(vec![
                        Level::new(dec!(80), dec!(1)),
                        Level::new(dec!(90), dec!(1)),
                        Level::new(dec!(100), dec!(1)),
                    ]),
                    new_level: Level::new(dec!(100), dec!(0)),
                    expected: OrderBookSide::bids(vec![
                        Level::new(dec!(80), dec!(1)),
                        Level::new(dec!(90), dec!(1)),
                    ]),
                },
                TestCase {
                    // TC1: Level exists & new value is > 0 => replace Level
                    book_side: OrderBookSide::bids(vec![
                        Level::new(dec!(80), dec!(1)),
                        Level::new(dec!(90), dec!(1)),
                        Level::new(dec!(100), dec!(1)),
                    ]),
                    new_level: Level::new(dec!(100), dec!(10)),
                    expected: OrderBookSide::bids(vec![
                        Level::new(dec!(80), dec!(1)),
                        Level::new(dec!(90), dec!(1)),
                        Level::new(dec!(100), dec!(10)),
                    ]),
                },
                TestCase {
                    // TC2: Level does not exist & new value > 0 => insert new Level
                    book_side: OrderBookSide::bids(vec![
                        Level::new(dec!(80), dec!(1)),
                        Level::new(dec!(90), dec!(1)),
                        Level::new(dec!(100), dec!(1)),
                    ]),
                    new_level: Level::new(dec!(110), dec!(1)),
                    expected: OrderBookSide::bids(vec![
                        Level::new(dec!(80), dec!(1)),
                        Level::new(dec!(90), dec!(1)),
                        Level::new(dec!(100), dec!(1)),
                        Level::new(dec!(110), dec!(1)),
                    ]),
                },
                TestCase {
                    // TC3: Level does not exist & new value is 0 => no change
                    book_side: OrderBookSide::bids(vec![
                        Level::new(dec!(80), dec!(1)),
                        Level::new(dec!(90), dec!(1)),
                        Level::new(dec!(100), dec!(1)),
                    ]),
                    new_level: Level::new(dec!(110), dec!(0)),
                    expected: OrderBookSide::bids(vec![
                        Level::new(dec!(80), dec!(1)),
                        Level::new(dec!(90), dec!(1)),
                        Level::new(dec!(100), dec!(1)),
                    ]),
                },
            ];

            for (index, mut test) in tests.into_iter().enumerate() {
                test.book_side.upsert_single(test.new_level, |existing| {
                    existing.price.cmp(&test.new_level.price).reverse()
                });
                assert_eq!(test.book_side, test.expected, "TC{} failed", index);
            }
        }
    }
}
