use crate::statistic::metric::MetricRolling;
use crate::portfolio::position::Position;
use chrono::{Duration, DateTime, Utc};
use crate::statistic::dispersion::Dispersion;
use crate::statistic::summary::Summariser;
use crate::statistic::algorithm::WelfordOnline;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct PnLReturnView {
    pub start_timestamp: DateTime<Utc>,
    pub duration: Duration,
    pub total: DataSummary,
    pub losses: DataSummary,
}

impl Summariser for PnLReturnView {
    const SUMMARY_ID: &'static str = "PnL Return Summary";

    fn update_summary(&mut self, position: &Position) {
        // Set start timestamp if it's the first trade of the session
        if self.total.count == 0 {
            self.start_timestamp = position.meta.enter_bar_timestamp;
        }

        // Update duration of trading session
        self.update_trading_session_duration(position);

        // Update Total PnL Returns
        self.total.update(position);

        // Update Loss PnL Returns if relevant
        if let Some(is_loss) = position.is_loss() {
            if is_loss {
                self.losses.update(position);
            }
        }
    }

    fn print_table(&self) {
        todo!()
    }
}

impl MetricRolling for PnLReturnView {
    const METRIC_ID: &'static str = "PnL Return Summary";

    fn init() -> Self {
        Self {
            start_timestamp: Utc::now(),
            duration: Duration::zero(),
            total: DataSummary::default(),
            losses: DataSummary::default()
        }
    }

    fn update(&mut self, position: &Position) {
    }
}

impl PnLReturnView {
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
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct DataSummary {
    pub count: usize,
    pub sum: f64,
    pub mean: f64,
    pub dispersion: Dispersion,
}

impl Default for DataSummary {
    fn default() -> Self {
        Self {
            count: 0,
            sum: 0.0,
            mean: 0.0,
            dispersion: Dispersion::default()
        }
    }
}

impl MetricRolling for DataSummary {
    const METRIC_ID: &'static str = "PnL Return";

    fn init() -> Self {
        Self {
            count: 0,
            sum: 0.0,
            mean: 0.0,
            dispersion: Dispersion::default(),
        }
    }

    fn update(&mut self, position: &Position) {
        // Increment trade counter
        self.count += 1;

        // Calculate next PnL Return data point
        let next_return = position.calculate_profit_loss_return();

        // Update Sum
        self.sum += next_return;

        // Update Mean
        let prev_mean = self.mean;
        self.mean = WelfordOnline::calculate_mean(self.mean, next_return, self.count);

        // Update Dispersion
        self.dispersion.update(prev_mean, self.mean, next_return, self.count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn pnl_return_summary_update() {
    //     // -- INPUT POSITIONS -- //
    //     let base_timestamp = Utc::now();
    //
    //     let mut input_position_1 = Position::default();  // PnL Return = 10
    //     input_position_1.meta.enter_bar_timestamp = base_timestamp;
    //     input_position_1.meta.exit_bar_timestamp = Some(base_timestamp.checked_add_signed(Duration::days(1)).unwrap());
    //     input_position_1.result_profit_loss = 1000.0;
    //     input_position_1.enter_value_gross = 100.0;
    //
    //     let mut input_position_2 = Position::default();  // PnL Return = 100
    //     input_position_2.meta.enter_bar_timestamp = base_timestamp.checked_add_signed(Duration::days(2)).unwrap();
    //     input_position_2.meta.exit_bar_timestamp = Some(base_timestamp.checked_add_signed(Duration::days(3)).unwrap());
    //     input_position_2.result_profit_loss = 10000.0;
    //     input_position_2.enter_value_gross = 100.0;
    //
    //     let mut input_position_3 = Position::default(); // PnL Return = -10
    //     input_position_3.meta.enter_bar_timestamp = base_timestamp.checked_add_signed(Duration::days(4)).unwrap();
    //     input_position_3.meta.exit_bar_timestamp = Some(base_timestamp.checked_add_signed(Duration::days(5)).unwrap());
    //     input_position_3.result_profit_loss = -1000.0;
    //     input_position_3.enter_value_gross = 100.0;
    //
    //     let input_positions = vec![input_position_1, input_position_2, input_position_3];
    //
    //     // -- ACTUAL GENERATED PnL RETURN SUMMARIES --
    //     let mut actual_pnl_summaries = Vec::new();
    //
    //     let mut pnl_summary = DataSummary::init();
    //     for position in input_positions {
    //         pnl_summary.update(&position);
    //         actual_pnl_summaries.push(pnl_summary.clone())
    //     }
    //
    //
    //     // -- EXPECTED PnL RETURN SUMMARIES -- //
    //     let pnl_return_1 = DataSummary {
    //         counter: 1,
    //         start_timestamp: base_timestamp,
    //         duration: Duration::days(1),
    //         sum: 10.0,
    //         mean: 10.0,
    //         recurrence_relation_m: 0.0,
    //         variance: 0.0,
    //         standard_deviation: 0.0
    //     };
    //
    //     let pnl_return_2 = DataSummary {
    //         counter: 2,
    //         start_timestamp: base_timestamp,
    //         duration: Duration::days(3),
    //         sum: 110.0,
    //         mean: 55.0,
    //         recurrence_relation_m: 4050.0,
    //         variance: 2025.0,
    //         standard_deviation: 45.0
    //     };
    //
    //     let pnl_return_3 = DataSummary {
    //         counter: 3,
    //         start_timestamp: base_timestamp,
    //         duration: Duration::days(5),
    //         sum: 100.0,
    //         mean: 33.33333333333333,
    //         recurrence_relation_m: 6866.666666666666,
    //         variance: 2288.8888888888887,
    //         standard_deviation: 47.84233364802441
    //     };
    //
    //     // -- ASSERT ACTUAL EQUALS EXPECTED --
    //     assert_eq!(actual_pnl_summaries[0], pnl_return_1);
    //     assert_eq!(actual_pnl_summaries[1], pnl_return_2);
    //     assert_eq!(actual_pnl_summaries[2], pnl_return_3);
    // }
}
