use crate::statistic::metric::drawdown::{Drawdown, AvgDrawdown, MaxDrawdown};
use crate::statistic::summary::trading::{PositionSummariser, TablePrinter};
use crate::portfolio::position::Position;
use prettytable::{Row, Table};

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct DrawdownSummary {
    pub current_drawdown: Drawdown,
    pub avg_drawdown: AvgDrawdown,
    pub max_drawdown: MaxDrawdown,
}

impl PositionSummariser for DrawdownSummary {
    fn update(&mut self, position: &Position) {
        // Only update DrawdownSummary with closed Positions
        let equity_point = match &position.meta.exit_equity_point {
            None => return,
            Some(equity_point) => equity_point,
        };

        // Updates
        if let Some(ended_drawdown) = self.current_drawdown.update(equity_point) {
            self.avg_drawdown.update(&ended_drawdown);
            self.max_drawdown.update(&ended_drawdown);
        }
    }
}

impl TablePrinter for DrawdownSummary {
    fn print(&self) {
        let mut drawdown_summary = Table::new();

        let titles = vec!["",
            "Max Drawdown", "Max Drawdown Duration", "Avg. Drawdown", "Avg. Drawdown Duration",
        ];

        drawdown_summary.add_row(row!["Total",
            self.max_drawdown.drawdown.drawdown, self.max_drawdown.drawdown.duration.to_string(),
            self.avg_drawdown.mean_drawdown, self.avg_drawdown.mean_duration.to_string()
        ]);

        drawdown_summary.set_titles(Row::from(titles));
        drawdown_summary.printstd();
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