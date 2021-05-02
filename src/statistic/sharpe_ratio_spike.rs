use crate::statistic::metric::MetricRolling;
use crate::portfolio::position::Position;

pub struct PnLReturnMean {
    counter: usize,
    mean: f64,
}

impl MetricRolling for PnLReturnMean {
    const METRIC_ID: &'static str = "Profit & Loss Return Mean";

    fn init() -> Self {
        Self {
            counter: 0,
            mean: 0.0
        }
    }

    fn update(&mut self, position: &Position) {
        // Update data set length counter
        self.counter += 1;

        // Calculate the PnL return of new Position data point
        let pnl_return = position.result_profit_loss / (position.enter_value_gross + position.enter_fees_total);

        // Calculate new mean
        self.mean += (pnl_return - self.mean) / self.counter as f64;
    }
}

pub struct PnLReturnStandardDeviation {
    counter: usize,
    standard_deviation: f64,
}

impl MetricRolling for PnLReturnStandardDeviation {
    const METRIC_ID: &'static str = "Profit & Loss Return Standard Deviation";

    fn init() -> Self {
        Self {
            counter: 0,
            standard_deviation: 0.0
        }
    }

    fn update(&mut self, position: &Position) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean() {
        let mut metric_mean = PnLReturnMean::init();

        // First Position PnL Return = 10;
        let mut first_position = Position::default();
        first_position.result_profit_loss = 1000.0;
        first_position.enter_value_gross = 100.0;

        // Second Position PnL Return = 100;
        let mut second_position = Position::default();
        second_position.result_profit_loss = 10000.0;
        second_position.enter_value_gross = 100.0;

        // Third Position PnL Return = 10;
        let mut third_position = Position::default();
        third_position.result_profit_loss = 1000.0;
        third_position.enter_value_gross = 100.0;

        // First Update Mean
        metric_mean.update(&first_position);
        let actual_first_mean = metric_mean.mean;
        let expected_first_mean = 10.0;                               // 10

        // Second Update Mean
        metric_mean.update(&second_position);
        let actual_second_mean = metric_mean.mean;
        let expected_second_mean = (10.0 + 100.0) / 2.0;              // 55

        // Second Update Mean
        metric_mean.update(&third_position);
        let actual_third_mean = metric_mean.mean;
        let expected_third_mean = (10.0 + 100.0 + 10.0) / 3.0;        // 40

        assert_eq!(actual_first_mean, expected_first_mean);
        assert_eq!(actual_second_mean, expected_second_mean);
        assert_eq!(actual_third_mean, expected_third_mean);
    }
}

