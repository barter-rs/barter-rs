use crate::{
    portfolio::position::Position,
    statistic::{
        de_duration_from_secs, se_duration_as_secs,
        summary::{data::DataSummary, Initialiser, PositionSummariser, TableBuilder},
    },
};
use barter_integration::model::Side;
use chrono::{DateTime, Duration, Utc};
use prettytable::Row;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PnLReturnSummary {
    pub time: DateTime<Utc>,
    #[serde(
        deserialize_with = "de_duration_from_secs",
        serialize_with = "se_duration_as_secs"
    )]
    pub duration: Duration,
    pub trades_per_day: f64,
    pub total: DataSummary,
    pub losses: DataSummary,
}

impl Initialiser for PnLReturnSummary {
    type Config = ();

    fn init(_: Self::Config) -> Self {
        Self::default()
    }
}

impl Default for PnLReturnSummary {
    fn default() -> Self {
        Self {
            time: Utc::now(),
            duration: Duration::zero(),
            trades_per_day: 0.0,
            total: DataSummary::default(),
            losses: DataSummary::default(),
        }
    }
}

impl PositionSummariser for PnLReturnSummary {
    fn update(&mut self, position: &Position) {
        // Set start timestamp if it's the first trade of the session
        if self.total.count == 0 {
            self.time = position.meta.enter_time;
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

impl TableBuilder for PnLReturnSummary {
    fn titles(&self) -> Row {
        row![
            "Trades",
            "Wins",
            "Losses",
            "Trading Days",
            "Trades Per Day",
            "Mean Return",
            "Std. Dev. Return",
            "Loss Mean Return",
            "Biggest Win",
            "Biggest Loss",
        ]
    }

    fn row(&self) -> Row {
        let wins = self.total.count - self.losses.count;
        row![
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
        ]
    }
}

impl PnLReturnSummary {
    const SECONDS_IN_DAY: f64 = 86400.0;

    pub fn new() -> Self {
        Self {
            time: Utc::now(),
            duration: Duration::zero(),
            trades_per_day: 0.0,
            total: Default::default(),
            losses: Default::default(),
        }
    }

    pub fn update_trading_session_duration(&mut self, position: &Position) {
        self.duration = match position.meta.exit_balance {
            None => {
                // Since Position is not exited, estimate duration w/ last_update_time
                position.meta.update_time.signed_duration_since(self.time)
            }
            Some(exit_balance) => exit_balance.time.signed_duration_since(self.time),
        }
    }

    pub fn update_trades_per_day(&mut self) {
        self.trades_per_day = self.total.count as f64
            / (self.duration.num_seconds() as f64 / PnLReturnSummary::SECONDS_IN_DAY)
    }
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Default, Deserialize, Serialize)]
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
        self.total_pnl += position.realised_profit_loss;
        self.total_pnl_per_contract = self.total_pnl / self.total_contracts;

        match position.side {
            Side::Buy => {
                self.long_contracts += position.quantity.abs();
                self.long_pnl += position.realised_profit_loss;
                self.long_pnl_per_contract = self.long_pnl / self.long_contracts;
            }
            Side::Sell => {
                self.short_contracts += position.quantity.abs();
                self.short_pnl += position.realised_profit_loss;
                self.short_pnl_per_contract = self.short_pnl / self.short_contracts;
            }
        }
    }
}

impl TableBuilder for ProfitLossSummary {
    fn titles(&self) -> Row {
        row![
            "Long Contracts",
            "Long PnL",
            "Long PnL Per Contract",
            "Short Contracts",
            "Short PnL",
            "Short PnL Per Contract",
            "Total Contracts",
            "Total PnL",
            "Total PnL Per Contract",
        ]
    }

    fn row(&self) -> Row {
        row![
            format!("{:.3}", self.long_contracts),
            format!("{:.3}", self.long_pnl),
            format!("{:.3}", self.long_pnl_per_contract),
            format!("{:.3}", self.short_contracts),
            format!("{:.3}", self.short_pnl),
            format!("{:.3}", self.short_pnl_per_contract),
            format!("{:.3}", self.total_contracts),
            format!("{:.3}", self.total_pnl),
            format!("{:.3}", self.total_pnl_per_contract),
        ]
    }
}

impl ProfitLossSummary {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{portfolio::Balance, test_util::position};
    use chrono::{Duration, Utc};

    #[test]
    fn update_pnl_return_summary() {
        // Todo:
    }

    #[test]
    fn update_trading_session_duration_with_non_exited_position() {
        let base_time = Utc::now();

        let mut pnl_return_view = PnLReturnSummary::new();
        pnl_return_view.time = base_time;

        let mut input_position = position();
        input_position.meta.exit_balance = None;
        input_position.meta.update_time = base_time.checked_add_signed(Duration::days(10)).unwrap();

        pnl_return_view.update_trading_session_duration(&input_position);

        let expected = Duration::days(10);

        assert_eq!(pnl_return_view.duration, expected);
    }

    #[test]
    fn update_trading_session_duration_with_exited_position() {
        let base_time = Utc::now();

        let mut pnl_return_view = PnLReturnSummary::new();
        pnl_return_view.time = base_time;

        let mut input_position = position();
        input_position.meta.exit_balance = Some(Balance {
            time: base_time.checked_add_signed(Duration::days(15)).unwrap(),
            total: 0.0,
            available: 0.0,
        });

        pnl_return_view.update_trading_session_duration(&input_position);

        let expected = Duration::days(15);

        assert_eq!(pnl_return_view.duration, expected);
    }
}
