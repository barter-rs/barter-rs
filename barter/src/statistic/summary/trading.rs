use crate::{
    portfolio::position::Position,
    statistic::{
        metric::ratio::{CalmarRatio, Ratio, SharpeRatio, SortinoRatio},
        summary::{
            drawdown::DrawdownSummary, pnl::PnLReturnSummary, Initialiser, PositionSummariser,
            TableBuilder,
        },
    },
};
use chrono::{DateTime, Duration, Utc};
use prettytable::{Cell, Row};
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
    pub pnl_returns: PnLReturnSummary,
    pub drawdown: DrawdownSummary,
    pub tear_sheet: TearSheet,
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

impl TableBuilder for TradingSummary {
    fn titles(&self) -> Row {
        let mut titles = Vec::<Cell>::new();

        for title in &self.pnl_returns.titles() {
            titles.push(title.clone())
        }

        for title in &self.tear_sheet.titles() {
            titles.push(title.clone())
        }

        for title in &self.drawdown.titles() {
            titles.push(title.clone())
        }

        Row::new(titles)
    }

    fn row(&self) -> Row {
        let mut cells = Vec::<Cell>::new();

        for cell in &self.pnl_returns.row() {
            cells.push(cell.clone())
        }

        for cell in &self.tear_sheet.row() {
            cells.push(cell.clone())
        }

        for cell in &self.drawdown.row() {
            cells.push(cell.clone())
        }

        Row::new(cells)
    }
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct TearSheet {
    pub sharpe_ratio: SharpeRatio,
    pub sortino_ratio: SortinoRatio,
    pub calmar_ratio: CalmarRatio,
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

impl TableBuilder for TearSheet {
    fn titles(&self) -> Row {
        row!["Sharpe Ratio", "Sortino Ratio", "Calmar Ratio"]
    }

    fn row(&self) -> Row {
        row![
            format!("{:.3}", self.sharpe_ratio.daily()),
            format!("{:.3}", self.sortino_ratio.daily()),
            format!("{:.3}", self.calmar_ratio.daily()),
        ]
    }
}

pub fn calculate_trading_duration(start_time: &DateTime<Utc>, position: &Position) -> Duration {
    match position.meta.exit_balance {
        None => {
            // Since Position is not exited, estimate duration w/ last_update_time
            position.meta.update_time.signed_duration_since(*start_time)
        }
        Some(exit_balance) => exit_balance.time.signed_duration_since(*start_time),
    }
}
