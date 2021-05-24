use crate::statistic::summary::trading::{PositionSummariser, TablePrinter};
use crate::portfolio::position::{Position, Direction};
use crate::statistic::summary::data::DataSummary;
use chrono::{Duration, DateTime, Utc};
use serde::{Deserialize, Serialize};
use prettytable::{Row, Table};

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct PnLReturnSummary {
    pub start_timestamp: DateTime<Utc>,
    pub duration: Duration,
    pub trades_per_day: f64,
    pub total: DataSummary,
    pub losses: DataSummary,
}

impl Default for PnLReturnSummary {
    fn default() -> Self {
        Self {
            start_timestamp: Utc::now(),
            duration: Duration::zero(),
            trades_per_day: 0.0,
            total: DataSummary::default(),
            losses: DataSummary::default()
        }
    }
}

impl PositionSummariser for PnLReturnSummary {
    fn update(&mut self, position: &Position) {
        // Set start timestamp if it's the first trade of the session
        if self.total.count == 0 {
            self.start_timestamp = position.meta.enter_bar_timestamp;
        }

        // Update duration of trading session & trades per day
        self.update_trading_session_duration(position);
        self.update_trades_per_day();

        // Calculate the Position PnL Return
        let pnl_return = position.calculate_profit_loss_return();

        // Update Total PnL Returns
        self.total.update(pnl_return);

        // Update Loss PnL Returns if relevant
        if pnl_return.is_sign_negative() {
            self.losses.update(pnl_return);
        }
    }
}

impl TablePrinter for PnLReturnSummary {
    fn print(&self) {
        let mut pnl_returns = Table::new();

        let titles = vec!["",
            "Trades", "Wins", "Losses", "Trading Days", "Trades Per Day",
            "Mean Return", "Std. Dev. Return", "Loss Mean Return",
            "Biggest Win", "Biggest Loss"];

        let wins = self.total.count - self.losses.count;

        pnl_returns.add_row(row!["Total",
            self.total.count.to_string(),
            wins,
            self.losses.count,
            self.duration.num_days().to_string(),
            format!("{:.3}", self.trades_per_day),
            format!("{:.3}", self.total.mean),
            format!("{:.3}", self.total.dispersion.std_dev),
            format!("{:.3}", self.losses.mean),
            format!("{:.3}", self.total.dispersion.range.high),
            format!("{:.3}", self.total.dispersion.range.low),
        ]);

        pnl_returns.set_titles(Row::from(titles));
        pnl_returns.printstd();
    }
}

impl PnLReturnSummary {
    const SECONDS_IN_DAY: f64 = 86400.0;

    pub fn new() -> Self {
        Self {
            start_timestamp: Utc::now(),
            duration: Duration::zero(),
            trades_per_day: 0.0,
            total: Default::default(),
            losses: Default::default()
        }
    }

    fn update_trading_session_duration(&mut self, position: &Position) {
        self.duration = match position.meta.exit_bar_timestamp {
            None => {
                // Since Position is not exited, estimate duration w/ last_update_timestamp
                position.meta.last_update_timestamp.signed_duration_since(self.start_timestamp)
            },
            Some(exit_timestamp) => {
                exit_timestamp.signed_duration_since(self.start_timestamp)
            }
        }
    }

    fn update_trades_per_day(&mut self) {
        self.trades_per_day = self.total.count as f64
            / (self.duration.num_seconds() as f64 / PnLReturnSummary::SECONDS_IN_DAY)
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct ProfitLossSummary {
    pub long_contracts: f64,
    pub long_pnl: f64,
    pub long_pnl_per_contract: f64,
    pub short_contracts: f64,
    pub short_pnl: f64,
    pub short_pnl_per_contract: f64,
    pub total_contracts: f64,
    pub total_pnl: f64,
    pub total_pnl_per_contract: f64,
}

impl PositionSummariser for ProfitLossSummary {
    fn update(&mut self, position: &Position) {
        self.total_contracts += position.quantity.abs();
        self.total_pnl += position.result_profit_loss;
        self.total_pnl_per_contract = self.total_pnl / self.total_contracts;

        match position.direction {
            Direction::Long => {
                self.long_contracts += position.quantity.abs();
                self.long_pnl += position.result_profit_loss;
                self.long_pnl_per_contract = self.long_pnl / self.long_contracts;
            }
            Direction::Short => {
                self.short_contracts += position.quantity.abs();
                self.short_pnl += position.result_profit_loss;
                self.short_pnl_per_contract = self.short_pnl / self.short_contracts;
            }
        }
    }
}

impl TablePrinter for ProfitLossSummary {
    fn print(&self) {
        todo!()
    }
}

impl ProfitLossSummary {
    pub fn new() -> Self {
        Self {
            long_contracts: 0.0,
            long_pnl: 0.0,
            long_pnl_per_contract: 0.0,
            short_contracts: 0.0,
            short_pnl: 0.0,
            short_pnl_per_contract: 0.0,
            total_contracts: 0.0,
            total_pnl: 0.0,
            total_pnl_per_contract: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Utc, Duration};

    #[test]
    fn update_pnl_return_summary() {
        // struct TestCase {
        //     input_position: Position,
        //     expected_summary: PnLReturnSummary,
        // }
    }

    #[test]
    fn update_trading_session_duration_with_non_exited_position() {
        let base_timestamp = Utc::now();

        let mut pnl_return_view = PnLReturnSummary::new();
        pnl_return_view.start_timestamp = base_timestamp;

        let mut input_position = Position::default();
        input_position.meta.exit_bar_timestamp = None;
        input_position.meta.last_update_timestamp = base_timestamp
            .checked_add_signed(Duration::days(10)).unwrap();

        pnl_return_view.update_trading_session_duration(&input_position);

        let expected = Duration::days(10);

        assert_eq!(pnl_return_view.duration, expected);
    }

    #[test]
    fn update_trading_session_duration_with_exited_position() {
        let base_timestamp = Utc::now();

        let mut pnl_return_view = PnLReturnSummary::new();
        pnl_return_view.start_timestamp = base_timestamp;

        let mut input_position = Position::default();
        input_position.meta.exit_bar_timestamp = Some(base_timestamp
            .checked_add_signed(Duration::days(15)).unwrap());

        pnl_return_view.update_trading_session_duration(&input_position);

        let expected = Duration::days(15);

        assert_eq!(pnl_return_view.duration, expected);
    }
}