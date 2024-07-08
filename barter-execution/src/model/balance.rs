use barter_integration::model::instrument::symbol::Symbol;
use serde::{Deserialize, Serialize};

/// [`Balance`] associated with a [`Symbol`].
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct SymbolBalance {
    pub symbol: Symbol,
    pub balance: Balance,
}

impl SymbolBalance {
    /// Construct a new [`SymbolBalance`] from a [`Symbol`] and it's associated [`Balance`].
    pub fn new<S>(symbol: S, balance: Balance) -> Self
    where
        S: Into<Symbol>,
    {
        Self {
            symbol: symbol.into(),
            balance,
        }
    }
}

/// Total and available balance values.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Balance {
    pub total: f64,
    pub available: f64,
}

impl Balance {
    /// Construct a new [`Balance`].
    pub fn new(total: f64, available: f64) -> Self {
        Self { total, available }
    }

    /// Calculate the used (`total` - `available`) balance.
    pub fn used(&self) -> f64 {
        self.total - self.available
    }

    /// Apply a [`BalanceDelta`] to this [`Balance`].
    pub fn apply(&mut self, delta: BalanceDelta) {
        self.total += delta.total;
        self.available += delta.available;
    }
}

/// Communicates a change to be applied to a [`Balance`];
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BalanceDelta {
    pub total: f64,
    pub available: f64,
}

impl BalanceDelta {
    /// Construct a new [`BalanceDelta`].
    pub fn new(total: f64, available: f64) -> Self {
        Self { total, available }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_used() {
        // No Balance is used
        let balance = Balance::new(10.0, 10.0);
        assert_eq!(balance.used(), 0.0);

        // All Balance is used
        let balance = Balance::new(10.0, 0.0);
        assert_eq!(balance.used(), balance.total);

        // Half Balance is used
        let balance = Balance::new(10.0, 5.0);
        assert_eq!(balance.used(), balance.available);
    }

    #[test]
    fn test_balance_apply_balance_delta() {
        struct TestCase {
            balance: Balance,
            input_delta: BalanceDelta,
            expected: Balance,
        }

        let tests = vec![
            TestCase {
                // TC0: Delta applies a negative total delta only
                balance: Balance::new(10.0, 0.0),
                input_delta: BalanceDelta::new(-10.0, 0.0),
                expected: Balance::new(0.0, 0.0),
            },
            TestCase {
                // TC1: Delta applies a negative available delta only
                balance: Balance::new(10.0, 10.0),
                input_delta: BalanceDelta::new(0.0, -10.0),
                expected: Balance::new(10.0, 0.0),
            },
            TestCase {
                // TC2: Delta applies a positive available delta only
                balance: Balance::new(10.0, 10.0),
                input_delta: BalanceDelta::new(0.0, 10.0),
                expected: Balance::new(10.0, 20.0),
            },
            TestCase {
                // TC3: Delta applies a positive available delta only
                balance: Balance::new(10.0, 10.0),
                input_delta: BalanceDelta::new(0.0, 10.0),
                expected: Balance::new(10.0, 20.0),
            },
            TestCase {
                // TC4: Delta applies a positive total & available delta
                balance: Balance::new(10.0, 10.0),
                input_delta: BalanceDelta::new(10.0, 10.0),
                expected: Balance::new(20.0, 20.0),
            },
            TestCase {
                // TC5: Delta applies a negative total & available delta
                balance: Balance::new(10.0, 10.0),
                input_delta: BalanceDelta::new(-10.0, -10.0),
                expected: Balance::new(0.0, 0.0),
            },
        ];

        for (index, mut test) in tests.into_iter().enumerate() {
            test.balance.apply(test.input_delta);
            assert_eq!(test.balance, test.expected, "TC{} failed", index);
        }
    }
}
