use crate::portfolio::position::Position;
use chrono::{DateTime, Duration, Utc};
use crate::statistic::metric::metric_pnl_return_spike::{SharpeRatio, SortinoRatio, Drawdown};
use crate::statistic::metric::ratio::{SharpeRatio, SortinoRatio, CalmarRatio};
use crate::statistic::metric::drawdown::Drawdown;
use crate::statistic::metric::sharpe_ratio_spike::PnLReturnView;
use crate::statistic::metric::summary_old::SummariserOld;

pub trait TablePrinter {
    fn print_table(&self);
}

pub trait Summariser {
    // fn generate_summary(positions: &Vec<Position>) -> Self;
    fn update_summary(&mut self, position: &Position);
    fn print(&self);
}

pub struct SessionSummary {
    pnl_returns: PnLReturnView,
    tear_sheet: TearSheet,
}

impl Summariser for SessionSummary {
    fn update_summary(&mut self, position: &Position) {
        self.pnl_returns.update_summary(position);
        self.tear_sheet.update(position, &self.pnl_returns);
    }

    fn print(&self) {
        println!("\n-- Tear Sheet --");
        self.meta.print_table();
    }
}

pub struct TearSheet {
    drawdown: Drawdown,
    sharpe_ratio: SharpeRatio,
    sortino_ratio: SortinoRatio,
    calmar_ratio: CalmarRatio,
}

impl TearSheet {
    pub fn new(risk_free_return: f64) -> Self {
        Self {
            drawdown: Drawdown::init(1.0), // Todo: remove starting equity here
            sharpe_ratio: SharpeRatio::init(risk_free_return),
            sortino_ratio: SortinoRatio::init(risk_free_return),
            calmar_ratio: CalmarRatio::init(risk_free_return),
        }
    }

    pub fn update(&mut self, position: &Position, pnl_return_view: &PnLReturnView) {
        self.drawdown.update(position);
        self.sharpe_ratio.update(pnl_return_view);
        self.sortino_ratio.update(pnl_return_view);
        self.calmar_ratio.update(pnl_return_view, self.drawdown.max_drawdown);
    }
}
