use crate::statistic::dispersion::Dispersion;
use crate::statistic::summary::trading::{PositionSummariser, TablePrinter};
use crate::portfolio::position::Position;
use crate::statistic::algorithm::WelfordOnline;

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

impl PositionSummariser for DataSummary {
    fn update(&mut self, position: &Position) {
        // Increment trade counter
        self.count += 1;

        // Calculate next PnL Return data point
        let next_return = position.calculate_profit_loss_return();

        // Update Sum
        self.sum += next_return;

        // Update Mean
        let prev_mean = self.mean;
        self.mean = WelfordOnline::calculate_mean(self.mean, next_return, self.count as f64);

        // Update Dispersion
        self.dispersion.update(prev_mean, self.mean, next_return, self.count);
    }
}

impl TablePrinter for DataSummary {
    fn print(&self) {
        todo!()
    }
}

impl DataSummary {
    fn new() -> Self {
        Self {
            count: 0,
            sum: 0.0,
            mean: 0.0,
            dispersion: Dispersion::default(),
        }
    }
}