use crate::portfolio::position::Position;
use serde::Deserialize;
use crate::statistic::summary::pnl::PnLReturnSummary;
use crate::statistic::metric::ratio::{SharpeRatio, SortinoRatio, Ratio};
use prettytable::{Row, Table};
use chrono::{DateTime, Utc, Duration};
use crate::statistic::summary::drawdown::DrawdownSummary;

pub trait PositionSummariser {
    fn update(&mut self, position: &Position);
    fn generate_summary(&mut self, positions: &Vec<Position>) {
        for position in positions.iter() {
            self.update(position)
        }
    }
}

pub trait TablePrinter {
    fn print(&self);
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub trading_days_per_year: usize,
    pub risk_free_return: f64,
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct TradingSummary {
    pnl_returns: PnLReturnSummary,
    drawdown: DrawdownSummary,
    tear_sheet: TearSheet,
}

impl PositionSummariser for TradingSummary {
    fn update(&mut self, position: &Position) {
        self.pnl_returns.update(position);
        self.tear_sheet.update(position, &self.pnl_returns);
    }
}

impl TablePrinter for TradingSummary {
    fn print(&self) {
        println!("\n-- Tear Sheet --");
        self.tear_sheet.print();
    }
}

impl TradingSummary {
    pub fn new(cfg: &Config) -> Self {
        Self {
            pnl_returns: PnLReturnSummary::new(),
            drawdown: DrawdownSummary {},
            tear_sheet: TearSheet::new(cfg.risk_free_return)
        }
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct TearSheet {
    // drawdown: Drawdown,
    sharpe_ratio: SharpeRatio,
    sortino_ratio: SortinoRatio,
    // calmar_ratio: CalmarRatio,
}

impl TearSheet {
    pub fn new(risk_free_return: f64) -> Self {
        Self {
            // drawdown: Drawdown::init(),
            sharpe_ratio: SharpeRatio::init(risk_free_return),
            sortino_ratio: SortinoRatio::init(risk_free_return),
            // calmar_ratio: CalmarRatio::init(risk_free_return),
        }
    }

    pub fn update(&mut self, position: &Position, pnl_return_view: &PnLReturnSummary) {
        // self.drawdown.update(position);
        self.sharpe_ratio.update(pnl_return_view);
        self.sortino_ratio.update(pnl_return_view);
        // self.calmar_ratio.update(pnl_return_view, self.drawdown.max_drawdown);
    }
}

impl TablePrinter for TearSheet {
    fn print(&self) {
        let mut tear_sheet = Table::new();

        // let titles = vec!["",
        //                   "Avg. Drawdown", "Avg. Drawdown Duration", "Max Drawdown", "Max Drawdown Duration",
        //                   "Sharpe Ratio", "Sortino Ratio", "Calmar Ratio"
        // ];

        let titles = vec!["", "Sharpe Ratio", "Sortino Ratio"];

        tear_sheet.add_row(row!["Total",
            // self.drawdown.avg_drawdown, self.drawdown.avg_drawdown_duration.to_string(),
            // self.drawdown.max_drawdown, self.drawdown.max_drawdown_duration.to_string(),
            self.sharpe_ratio.daily().to_string(),
            self.sortino_ratio.daily().to_string(),
            // self.calmar_ratio.calculate_daily().to_string(),
        ]);

        tear_sheet.set_titles(Row::from(titles));
        tear_sheet.printstd();
    }
}

pub fn calculate_trading_duration(start_timestamp: &DateTime<Utc>, position: &Position) -> Duration {
    match position.meta.exit_bar_timestamp {
        None => {
            // Since Position is not exited, estimate duration w/ last_update_timestamp
            position.meta.last_update_timestamp.signed_duration_since(start_timestamp.clone())
        },
        Some(exit_timestamp) => {
            exit_timestamp.signed_duration_since(start_timestamp.clone())
        }
    }
}