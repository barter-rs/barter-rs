use crate::statistic::metric::drawdown::{Drawdown, AvgDrawdown, MaxDrawdown, EquityPoint};
use crate::statistic::summary::trading::PositionSummariser;
use crate::portfolio::position::Position;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct DrawdownSummary {
    pub current_drawdown: Drawdown,
    pub avg_drawdown: AvgDrawdown,
    pub max_drawdown: MaxDrawdown,
}

impl PositionSummariser for DrawdownSummary {
    fn update(&mut self, position: &Position) {

    }
}

impl DrawdownSummary {
    pub fn new(starting_equity: f64) -> Self {
        Self {
            current_drawdown: Drawdown::init(starting_equity),
            avg_drawdown: AvgDrawdown::init(),
            max_drawdown: MaxDrawdown::init(),
        }
    }
}