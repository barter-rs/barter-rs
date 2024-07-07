use crate::statistic::summary::pnl::PnLReturnSummary;
use serde::{Deserialize, Serialize};

pub trait Ratio {
    fn init(risk_free_return: f64) -> Self;
    fn ratio(&self) -> f64;
    fn trades_per_day(&self) -> f64;
    fn daily(&self) -> f64 {
        calculate_daily(self.ratio(), self.trades_per_day())
    }
    fn annual(&self, trading_days: u32) -> f64 {
        calculate_annual(self.ratio(), self.trades_per_day(), trading_days)
    }
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct SharpeRatio {
    pub risk_free_return: f64,
    pub trades_per_day: f64,
    pub sharpe_ratio_per_trade: f64,
}

impl Ratio for SharpeRatio {
    fn init(risk_free_return: f64) -> Self {
        Self {
            risk_free_return,
            sharpe_ratio_per_trade: 0.0,
            trades_per_day: 0.0,
        }
    }

    fn ratio(&self) -> f64 {
        self.sharpe_ratio_per_trade
    }

    fn trades_per_day(&self) -> f64 {
        self.trades_per_day
    }
}

impl SharpeRatio {
    pub fn update(&mut self, pnl_returns: &PnLReturnSummary) {
        // Update Trades Per Day
        self.trades_per_day = pnl_returns.trades_per_day;

        // Calculate Sharpe Ratio Per Trade
        self.sharpe_ratio_per_trade = match pnl_returns.total.dispersion.std_dev == 0.0 {
            true => 0.0,
            false => {
                (pnl_returns.total.mean - self.risk_free_return)
                    / pnl_returns.total.dispersion.std_dev
            }
        };
    }
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct SortinoRatio {
    pub risk_free_return: f64,
    pub trades_per_day: f64,
    pub sortino_ratio_per_trade: f64,
}

impl Ratio for SortinoRatio {
    fn init(risk_free_return: f64) -> Self {
        Self {
            risk_free_return,
            trades_per_day: 0.0,
            sortino_ratio_per_trade: 0.0,
        }
    }

    fn ratio(&self) -> f64 {
        self.sortino_ratio_per_trade
    }

    fn trades_per_day(&self) -> f64 {
        self.trades_per_day
    }
}

impl SortinoRatio {
    pub fn update(&mut self, pnl_returns: &PnLReturnSummary) {
        // Update Trades Per Day
        self.trades_per_day = pnl_returns.trades_per_day;

        // Calculate Sortino Ratio Per Trade
        self.sortino_ratio_per_trade = match pnl_returns.losses.dispersion.std_dev == 0.0 {
            true => 0.0,
            false => {
                (pnl_returns.total.mean - self.risk_free_return)
                    / pnl_returns.losses.dispersion.std_dev
            }
        };
    }
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CalmarRatio {
    pub risk_free_return: f64,
    pub trades_per_day: f64,
    pub calmar_ratio_per_trade: f64,
}

impl Ratio for CalmarRatio {
    fn init(risk_free_return: f64) -> Self {
        Self {
            risk_free_return,
            trades_per_day: 0.0,
            calmar_ratio_per_trade: 0.0,
        }
    }

    fn ratio(&self) -> f64 {
        self.calmar_ratio_per_trade
    }

    fn trades_per_day(&self) -> f64 {
        self.trades_per_day
    }
}

impl CalmarRatio {
    pub fn update(&mut self, pnl_returns: &PnLReturnSummary, max_drawdown: f64) {
        // Update Trades Per Day
        self.trades_per_day = pnl_returns.trades_per_day;

        // Calculate Calmar Ratio Per Trade
        self.calmar_ratio_per_trade = match max_drawdown == 0.0 {
            true => 0.0,
            false => (pnl_returns.total.mean - self.risk_free_return) / max_drawdown.abs(),
        };
    }
}

pub fn calculate_daily(ratio_per_trade: f64, trades_per_day: f64) -> f64 {
    ratio_per_trade * trades_per_day.sqrt()
}

pub fn calculate_annual(ratio_per_trade: f64, trades_per_day: f64, trading_days: u32) -> f64 {
    calculate_daily(ratio_per_trade, trades_per_day) * (trading_days as f64).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::statistic::summary::pnl::PnLReturnSummary;

    fn sharpe_ratio_input(count: u64, mean: f64, std_dev: f64) -> PnLReturnSummary {
        let mut pnl_returns = PnLReturnSummary::new();
        pnl_returns.total.count = count;
        pnl_returns.total.mean = mean;
        pnl_returns.total.dispersion.std_dev = std_dev;
        pnl_returns
    }

    fn sortino_update_input(count: u64, mean: f64, loss_std_dev: f64) -> PnLReturnSummary {
        let mut pnl_returns = PnLReturnSummary::new();
        pnl_returns.total.count = count;
        pnl_returns.total.mean = mean;
        pnl_returns.losses.dispersion.std_dev = loss_std_dev;
        pnl_returns
    }

    fn calmar_ratio_returns_input(count: u64, mean: f64) -> PnLReturnSummary {
        let mut pnl_returns = PnLReturnSummary::new();
        pnl_returns.total.count = count;
        pnl_returns.total.mean = mean;
        pnl_returns
    }

    #[test]
    fn sharpe_ratio_update() {
        let mut sharpe = SharpeRatio::init(0.0);

        struct TestCase {
            input_return: PnLReturnSummary,
            expected_sharpe: f64,
        }

        // Returns  = [0.1, 0.2, 0.3, 0.4, -0.4]
        // Means    = [0.1, 0.15, 0.2, 0.25, 0.12]
        // Std. Dev = [0.0, 0.05, (1/150).sqrt(), (0.0125).sqrt(), (0.388/5).sqrt()]
        let test_cases = vec![
            TestCase {
                // Test case 0: 1st trade, 10% profit
                input_return: sharpe_ratio_input(1, 0.1, 0.0),
                expected_sharpe: 0.0,
            },
            TestCase {
                // Test case 1: 2nd trade, 20% profit
                input_return: sharpe_ratio_input(2, 0.15, 0.05),
                expected_sharpe: 3.0,
            },
            TestCase {
                // Test case 2: 3rd trade, 30% profit
                input_return: sharpe_ratio_input(3, 0.2, (1.0_f64 / 150.0_f64).sqrt()),
                expected_sharpe: 6.0_f64.sqrt(),
            },
            TestCase {
                // Test case 3: 4th trade, 40% profit
                input_return: sharpe_ratio_input(4, 0.25, (0.0125_f64).sqrt()),
                expected_sharpe: 5.0_f64.sqrt(),
            },
            TestCase {
                // Test case 4: 5th trade, -40% profit
                input_return: sharpe_ratio_input(5, 0.12, (0.388_f64 / 5.0_f64).sqrt()),
                expected_sharpe: ((3.0 * 194_f64.sqrt()) / 97.0),
            },
        ];

        for (index, test) in test_cases.into_iter().enumerate() {
            sharpe.update(&test.input_return);
            let sharpe_diff = sharpe.sharpe_ratio_per_trade - test.expected_sharpe;
            assert!(sharpe_diff < 1e-10, "Test case: {:?}", index);
        }
    }

    #[test]
    fn sortino_ratio_update() {
        let mut sortino = SortinoRatio::init(0.0);

        struct TestCase {
            input_return: PnLReturnSummary,
            expected_sortino: f64,
        }

        // Returns       = [0.1, 0.2, 0.3, 0.4, -0.4, -0.6, -0.7]
        // Means         = [0.1, 0.15, 0.2, 0.25, 0.12, 0.0, -0.1]
        // Loss Std. Dev = [0.0, 0.0, 0.0, 0.0, 0.0, 0.1, 0.12472191]

        let test_cases = vec![
            TestCase {
                // Test case 0: 1st trade, 10% profit
                input_return: sortino_update_input(1, 0.1, 0.0),
                expected_sortino: 0.0,
            },
            TestCase {
                // Test case 1: 2nd trade, 20% profit
                input_return: sortino_update_input(2, 0.15, 0.0),
                expected_sortino: 0.0,
            },
            TestCase {
                // Test case 2: 3rd trade, 30% profit
                input_return: sortino_update_input(3, 0.2, 0.0),
                expected_sortino: 0.0,
            },
            TestCase {
                // Test case 3: 4th trade, 40% profit
                input_return: sortino_update_input(4, 0.25, 0.0),
                expected_sortino: 0.0,
            },
            TestCase {
                // Test case 4: 5th trade, -40% profit
                input_return: sortino_update_input(5, 0.12, 0.0),
                expected_sortino: 0.0,
            },
            TestCase {
                // Test case 5: 6th trade, -60% profit
                input_return: sortino_update_input(6, 0.0, 0.1),
                expected_sortino: 0.0,
            },
            TestCase {
                // Test case 5: 6th trade, -70% profit
                input_return: sortino_update_input(7, -0.1, 0.12472191),
                expected_sortino: -0.8017837443,
            },
        ];

        for (index, test) in test_cases.into_iter().enumerate() {
            sortino.update(&test.input_return);
            let sortino_diff = sortino.sortino_ratio_per_trade - test.expected_sortino;
            assert!(sortino_diff < 1e-10, "Test case: {:?}", index);
        }
    }

    #[test]
    fn calmar_ratio_update() {
        let mut calmar = CalmarRatio::init(0.0);

        struct TestCase {
            input_return: PnLReturnSummary,
            input_max_dd: f64,
            expected_calmar: f64,
        }

        // Returns       = [0.5, -0.7, 0.8, 1.4, -0.8]
        // Means         = [0.5, -0.1, 0.2, 0.5, 0.24]
        // Equity Points = [1.5, 0.45, 0.81, 1.944, 0.3888] (highest= 1.944, lowest after highest = 0.3888)
        // Max Drawdown  = [0.0, -0.7, -0.7, -0.7, -0.8]
        let test_cases = vec![
            TestCase {
                // Test case 0
                input_return: calmar_ratio_returns_input(1, 0.5),
                input_max_dd: 0.0,
                expected_calmar: 0.0,
            },
            TestCase {
                // Test case 1
                input_return: calmar_ratio_returns_input(2, -0.5),
                input_max_dd: -0.70,
                expected_calmar: (-0.1 / 0.7),
            },
            TestCase {
                // Test case 2
                input_return: calmar_ratio_returns_input(3, 0.2),
                input_max_dd: -0.7,
                expected_calmar: (0.2 / 0.7),
            },
            TestCase {
                // Test case 3
                input_return: calmar_ratio_returns_input(4, 0.5),
                input_max_dd: -0.7,
                expected_calmar: (0.5 / 0.7),
            },
            TestCase {
                // Test case 4
                input_return: calmar_ratio_returns_input(5, 0.24),
                input_max_dd: -0.8,
                expected_calmar: (0.24 / 0.8),
            },
        ];

        for (index, test) in test_cases.into_iter().enumerate() {
            calmar.update(&test.input_return, test.input_max_dd);
            let calmar_diff = calmar.calmar_ratio_per_trade - test.expected_calmar;
            assert!(calmar_diff < 1e-10, "Test case: {:?}", index);
        }
    }

    #[test]
    fn calculate_daily_ratios() {
        struct TestCase {
            ratio_per_trade: f64,
            trades_per_day: f64,
            expected_daily: f64,
        }

        let test_cases = vec![
            TestCase {
                ratio_per_trade: -1.0,
                trades_per_day: 0.1,
                expected_daily: -0.31622776601683794,
            },
            TestCase {
                ratio_per_trade: -1.0,
                trades_per_day: 1.0,
                expected_daily: -1.0,
            },
            TestCase {
                ratio_per_trade: 0.0,
                trades_per_day: 0.1,
                expected_daily: 0.0,
            },
            TestCase {
                ratio_per_trade: 0.0,
                trades_per_day: 1.0,
                expected_daily: 0.0,
            },
            TestCase {
                ratio_per_trade: 1.0,
                trades_per_day: 0.1,
                expected_daily: 0.31622776601683794,
            },
            TestCase {
                ratio_per_trade: 1.0,
                trades_per_day: 1.0,
                expected_daily: 1.0,
            },
            TestCase {
                ratio_per_trade: 100.0,
                trades_per_day: 0.1,
                expected_daily: 31.622776601683793,
            },
            TestCase {
                ratio_per_trade: 100.0,
                trades_per_day: 1.0,
                expected_daily: 100.0,
            },
        ];

        for test in test_cases {
            let actual = calculate_daily(test.ratio_per_trade, test.trades_per_day);
            assert_eq!(actual, test.expected_daily)
        }
    }

    #[test]
    fn calculate_annual_ratios() {
        struct TestCase {
            ratio_per_trade: f64,
            trades_per_day: f64,
            trading_days: u32,
            expected_annual: f64,
        }

        let test_cases = vec![
            TestCase {
                ratio_per_trade: -1.0,
                trades_per_day: 0.1,
                trading_days: 252,
                expected_annual: -5.019960159204453,
            },
            TestCase {
                ratio_per_trade: -1.0,
                trades_per_day: 1.0,
                trading_days: 365,
                expected_annual: -19.1049731745428,
            },
            TestCase {
                ratio_per_trade: 0.0,
                trades_per_day: 0.1,
                trading_days: 252,
                expected_annual: 0.0,
            },
            TestCase {
                ratio_per_trade: 0.0,
                trades_per_day: 1.0,
                trading_days: 365,
                expected_annual: 0.0,
            },
            TestCase {
                ratio_per_trade: 1.0,
                trades_per_day: 0.1,
                trading_days: 252,
                expected_annual: 5.019960159204453,
            },
            TestCase {
                ratio_per_trade: 1.0,
                trades_per_day: 1.0,
                trading_days: 365,
                expected_annual: 19.1049731745428,
            },
            TestCase {
                ratio_per_trade: 100.0,
                trades_per_day: 0.1,
                trading_days: 252,
                expected_annual: 501.99601592044536,
            },
            TestCase {
                ratio_per_trade: 100.0,
                trades_per_day: 1.0,
                trading_days: 365,
                expected_annual: 1910.49731745428,
            },
        ];

        for test in test_cases {
            let actual =
                calculate_annual(test.ratio_per_trade, test.trades_per_day, test.trading_days);
            assert_eq!(actual, test.expected_annual)
        }
    }
}
