use crate::portfolio::position::Position;
use crate::statistic::metric::ratio::{CalmarRatio, Ratio, SharpeRatio, SortinoRatio};
use crate::statistic::summary::drawdown::DrawdownSummary;
use crate::statistic::summary::pnl::PnLReturnSummary;
use crate::statistic::summary::{PositionSummariser, TablePrinter};
use chrono::{DateTime, Duration, Utc};
use prettytable::{Row, Table};
use serde::Deserialize;

/// Configuration for construction a [TradingSummary] via the new() constructor method.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub starting_equity: f64,
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
        self.drawdown.update(position);
        self.tear_sheet.update(&self.pnl_returns, &self.drawdown);
    }
}

impl TablePrinter for TradingSummary {
    fn print(&self) {
        println!("\n-- Trades & Returns --");
        self.pnl_returns.print();

        println!("\n-- Drawdown --");
        self.drawdown.print();

        println!("\n-- Tear Sheet --");
        self.tear_sheet.print();
    }
}

impl TradingSummary {
    /// Constructs a new [TradingSummary].
    pub fn new(cfg: &Config) -> Self {
        Self {
            pnl_returns: PnLReturnSummary::new(),
            drawdown: DrawdownSummary::new(cfg.starting_equity),
            tear_sheet: TearSheet::new(cfg.risk_free_return),
        }
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct TearSheet {
    sharpe_ratio: SharpeRatio,
    sortino_ratio: SortinoRatio,
    calmar_ratio: CalmarRatio,
}

impl TearSheet {
    pub fn new(risk_free_return: f64) -> Self {
        Self {
            sharpe_ratio: SharpeRatio::init(risk_free_return),
            sortino_ratio: SortinoRatio::init(risk_free_return),
            calmar_ratio: CalmarRatio::init(risk_free_return),
        }
    }

    pub fn update(&mut self, pnl_returns: &PnLReturnSummary, drawdown: &DrawdownSummary) {
        self.sharpe_ratio.update(pnl_returns);
        self.sortino_ratio.update(pnl_returns);
        self.calmar_ratio
            .update(pnl_returns, drawdown.max_drawdown.drawdown.drawdown);
    }
}

impl TablePrinter for TearSheet {
    fn print(&self) {
        let mut tear_sheet = Table::new();

        let titles = vec!["", "Sharpe Ratio", "Sortino Ratio", "Calmar Ratio"];

        tear_sheet.add_row(row![
            "Total",
            format!("{:.3}", self.sharpe_ratio.daily()),
            format!("{:.3}", self.sortino_ratio.daily()),
            format!("{:.3}", self.calmar_ratio.daily()),
        ]);

        tear_sheet.set_titles(Row::from(titles));
        tear_sheet.printstd();
    }
}

pub fn calculate_trading_duration(
    start_timestamp: &DateTime<Utc>,
    position: &Position,
) -> Duration {
    match position.meta.exit_bar_timestamp {
        None => {
            // Since Position is not exited, estimate duration w/ last_update_timestamp
            position
                .meta
                .last_update_timestamp
                .signed_duration_since(start_timestamp.clone())
        }
        Some(exit_timestamp) => exit_timestamp.signed_duration_since(start_timestamp.clone()),
    }
}
