use crate::portfolio::position::Position;
use crate::statistic::metric::ratio::{CalmarRatio, Ratio, SharpeRatio, SortinoRatio};
use crate::statistic::summary::drawdown::DrawdownSummary;
use crate::statistic::summary::pnl::PnLReturnSummary;
use crate::statistic::summary::{Initialiser, PositionSummariser, TablePrinter};
use chrono::{DateTime, Duration, Utc};
use prettytable::{Row, Table};
use serde::{Deserialize, Serialize};

/// Configuration for initialising a [`TradingSummary`] via the init() constructor method.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Config {
    pub starting_equity: f64,
    pub trading_days_per_year: usize,
    pub risk_free_return: f64,
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct TradingSummary {
    pnl_returns: PnLReturnSummary,
    drawdown: DrawdownSummary,
    tear_sheet: TearSheet,
}

impl Initialiser for TradingSummary {
    type Config = Config;

    fn init(config: Self::Config) -> Self {
        Self {
            pnl_returns: PnLReturnSummary::new(),
            drawdown: DrawdownSummary::new(config.starting_equity),
            tear_sheet: TearSheet::new(config.risk_free_return),
        }
    }
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

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
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
        self.calmar_ratio.update(pnl_returns, drawdown.max_drawdown.drawdown.drawdown);
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
    match position.meta.exit_balance {
        None => {
            // Since Position is not exited, estimate duration w/ last_update_timestamp
            position
                .meta
                .last_update_timestamp
                .signed_duration_since(*start_timestamp)
        }
        Some(exit_balance) => {
            exit_balance.timestamp.signed_duration_since(*start_timestamp)
        }
    }
}
