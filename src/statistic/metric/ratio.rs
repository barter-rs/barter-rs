use crate::statistic::summary::pnl::PnLReturnSummary;

pub trait Ratio {
    fn init(risk_free_return: f64) -> Self;
    fn ratio(&self) -> f64;
    fn trades_per_day(&self) -> f64;
    fn daily(&self) -> f64 {
        calculate_daily(self.ratio(), self.trades_per_day())
    }
    fn annual(&self, trading_days: usize) -> f64 {
        calculate_annual(self.ratio(), self.trades_per_day(), trading_days)
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
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
            true => {
                0.0
            }
            false => {
                (pnl_returns.total.mean - self.risk_free_return)
                    / pnl_returns.total.dispersion.std_dev
            }
        };
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
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
            sortino_ratio_per_trade: 0.0
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
            true => {
                0.0
            }
            false => {
                (pnl_returns.total.mean - self.risk_free_return)
                    / pnl_returns.losses.dispersion.std_dev
            }
        };
    }
}

// #[derive(Debug, Clone, PartialOrd, PartialEq)]
// pub struct CalmarRatio {
//     pub risk_free_return: f64,
//     pub trades_per_day: f64,
//     pub calmar_ratio_per_trade: f64,
// }
//
// impl Ratio for CalmarRatio {
//     fn init(risk_free_return: f64) -> Self {
//         Self {
//             risk_free_return,
//             trades_per_day: 0.0,
//             calmar_ratio_per_trade: 0.0
//         }
//     }
//
//     fn ratio(&self) -> f64 {
//         self.calmar_ratio_per_trade
//     }
//
//     fn trades_per_day(&self) -> f64 {
//         self.trades_per_day
//     }
// }
//
// impl CalmarRatio {
//     pub fn update(&mut self, pnl_returns: &PnLReturnSummary, max_drawdown: f64) {
//         // Update Trades Per Day
//         self.trades_per_day = pnl_returns.trades_per_day;
//
//         // Calculate Calmar Ratio Per Trade // Todo: Ensure max_drawdown isn't periodised...
//         self.calmar_ratio_per_trade = match max_drawdown == 0.0 {
//             true => 0.0,
//             false => (pnl_returns.total.mean - self.risk_free_return) / max_drawdown,
//         };
//     }
// }

pub fn calculate_daily(ratio_per_trade: f64, trades_per_day: f64) -> f64 {
    ratio_per_trade * trades_per_day.sqrt()
}

pub fn calculate_annual(ratio_per_trade: f64, trades_per_day: f64, trading_days: usize) -> f64 {
    calculate_daily(ratio_per_trade, trades_per_day) * (trading_days as f64).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::statistic::summary::pnl::PnLReturnSummary;
    use chrono::Duration;

    fn sharpe_update_input(count: usize, mean: f64, duration: Duration, std_dev: f64) -> PnLReturnSummary {
        let mut pnl_returns = PnLReturnSummary::new();
        pnl_returns.total.count = count;
        pnl_returns.total.mean = mean;
        pnl_returns.duration = duration;
        pnl_returns.total.dispersion.std_dev = std_dev;
        pnl_returns
    }

    fn sortino_update_input(count: usize, mean: f64, duration: Duration, loss_std_dev: f64) -> PnLReturnSummary {
        let mut pnl_returns = PnLReturnSummary::new();
        pnl_returns.total.count = count;
        pnl_returns.total.mean = mean;
        pnl_returns.duration = duration;
        pnl_returns.losses.dispersion.std_dev = loss_std_dev;
        pnl_returns
    }

    #[test]
    fn sharpe_ratio_update() {
        let mut sharpe = SharpeRatio::init(0.0);

        // Dataset  = [1.1, 1.2, 1.3, 1.4, 0.6]
        // Means    = [1.1, 1.15, 1.2, 1.25, 1.12]
        // Std. Dev = [0.0, 0.05, ~(6.sqrt()/30), ~(5.sqrt()/20), ~(194.sqrt()/50)]
        let inputs = vec![
            sharpe_update_input(1, 1.1, Duration::days(10), 0.0),                     // 1st trade, 10% profit
            sharpe_update_input(2, 1.15, Duration::days(10), 0.05),                   // 2nd trade, 20% profit
            sharpe_update_input(3, 1.2, Duration::days(10), 6.0_f64.sqrt()/30.0),     // 3rd trade, 30% profit
            sharpe_update_input(4, 1.25, Duration::days(10), 5.0_f64.sqrt()/20.0),    // 4th trade, 40% profit
            sharpe_update_input(5, 1.12, Duration::days(10), 194.0_f64.sqrt()/50.0),  // 5th trade, -40% profit
        ];

        let outputs = vec![
            0.0, 23.0, (6.0 * 6.0_f64.sqrt()), (5.0 * 5.0_f64.sqrt()), ((28.0 * 194_f64.sqrt())/97.0)
        ];

        for (input, out) in inputs.into_iter().zip(outputs.into_iter()) {
            sharpe.update(&input);
            let sharpe_diff = sharpe.sharpe_ratio_per_trade - out;
            assert!(sharpe_diff < 1e-10);
        }
    }

    #[test]
    fn sortino_ratio_update() {
        let mut sortino = SortinoRatio::init(0.0);

        // Dataset       = [1.1, 1.2, 1.3, 1.4, 0.6, 0.4]
        // Means         = [1.1, 1.15, 1.2, 1.25, 1.12, 1.0]
        // Loss Std. Dev = [0.0, 0.0, 0.0, 0.0, 0.0, 0.1]
        let inputs = vec![
            sortino_update_input(1, 1.1, Duration::days(10), 0.0),   // 1st trade, 10% profit
            sortino_update_input(2, 1.15, Duration::days(10), 0.0),  // 2nd trade, 20% profit
            sortino_update_input(3, 1.2, Duration::days(10), 0.0),   // 3rd trade, 30% profit
            sortino_update_input(4, 1.25, Duration::days(10), 0.0),  // 4th trade, 40% profit
            sortino_update_input(5, 1.12, Duration::days(10), 0.0),  // 5th trade, -40% profit
            sortino_update_input(6, 1.0, Duration::days(10), 0.1),   // 6th trade, -60% profit
        ];

        let outputs = vec![0.0, 0.0, 0.0, 0.0, 0.0, 10.0];

        for (input, out) in inputs.into_iter().zip(outputs.into_iter()) {
            sortino.update(&input);
            let sortino_diff = sortino.sortino_ratio_per_trade - out;
            assert!(sortino_diff < 1e-10);
        }
    }

    #[test]
    fn calmar_ratio_update() {
        // let mut calmar = CalmarRatio::init(0.0);
        //
        // // Dataset       = [1.1, 0.5, 1.4, 0.2, 2.0]
        // // Means         = [1.1, 0.8, 1.0, 0.8, 1.04]
        // // Equity Points = [1.1, 0.55, 0.77, 0.154, 0.308] (highest= 1.1)
        // // Max Drawdown  = [0.0, 0.55, 0.55, 0.0, 0.0, 0.1]
        //
        // let mut input_pnl_returns_1 = PnLReturnView::init();
        // input_pnl_returns_1.total.count = 1;                    // 1st trade, 10% profit
        // input_pnl_returns_1.total.mean = 1.1;
        // input_pnl_returns_1.duration = Duration::days(10);
        // let input_max_drawdown_1 = 0.0;
        //
        // let mut input_pnl_returns_2 = PnLReturnView::init();
        // input_pnl_returns_2.total.count = 1;                    // 2nd trade, -50% profit
        // input_pnl_returns_2.total.mean = 0.8;
        // input_pnl_returns_2.duration = Duration::days(10);
        // let input_max_drawdown_1 = 0.5;
    }

    #[test]
    fn calculate_daily_ratios() {
        struct TestCase {
            ratio_per_trade: f64,
            trades_per_day: f64,
            expected_daily: f64,
        }

        let test_cases = vec![
            TestCase { ratio_per_trade: -1.0, trades_per_day: 0.1, expected_daily: -0.31622776601683794 },
            TestCase { ratio_per_trade: -1.0, trades_per_day: 1.0, expected_daily: -1.0 },
            TestCase { ratio_per_trade: 0.0, trades_per_day: 0.1, expected_daily: 0.0 },
            TestCase { ratio_per_trade: 0.0, trades_per_day: 1.0, expected_daily: 0.0 },
            TestCase { ratio_per_trade: 1.0, trades_per_day: 0.1, expected_daily: 0.31622776601683794 },
            TestCase { ratio_per_trade: 1.0, trades_per_day: 1.0, expected_daily: 1.0 },
            TestCase { ratio_per_trade: 100.0, trades_per_day: 0.1, expected_daily: 31.622776601683793 },
            TestCase { ratio_per_trade: 100.0, trades_per_day: 1.0, expected_daily: 100.0 },
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
            trading_days: usize,
            expected_annual: f64,
        }

        let test_cases = vec![
            TestCase { ratio_per_trade: -1.0, trades_per_day: 0.1, trading_days: 252, expected_annual: -5.019960159204453 },
            TestCase { ratio_per_trade: -1.0, trades_per_day: 1.0, trading_days: 365, expected_annual: -19.1049731745428 },
            TestCase { ratio_per_trade: 0.0, trades_per_day: 0.1, trading_days: 252, expected_annual: 0.0 },
            TestCase { ratio_per_trade: 0.0, trades_per_day: 1.0, trading_days: 365, expected_annual: 0.0 },
            TestCase { ratio_per_trade: 1.0, trades_per_day: 0.1, trading_days: 252, expected_annual: 5.019960159204453 },
            TestCase { ratio_per_trade: 1.0, trades_per_day: 1.0, trading_days: 365, expected_annual: 19.1049731745428 },
            TestCase { ratio_per_trade: 100.0, trades_per_day: 0.1, trading_days: 252, expected_annual: 501.99601592044536 },
            TestCase { ratio_per_trade: 100.0, trades_per_day: 1.0, trading_days: 365, expected_annual: 1910.49731745428 },
        ];

        for test in test_cases {
            let actual = calculate_annual(test.ratio_per_trade, test.trades_per_day, test.trading_days);
            assert_eq!(actual, test.expected_annual)
        }
    }
}