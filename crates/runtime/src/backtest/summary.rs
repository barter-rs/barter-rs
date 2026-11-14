use crate::statistic::summary::TradingSummary;
use rust_decimal::Decimal;
use smol_str::SmolStr;
use std::time::Duration;

/// Container for multiple [`BacktestSummary`]s and associated multi backtest metadata.
#[derive(Debug)]
pub struct MultiBacktestSummary<Interval> {
    /// Number of backtests run in this batch.
    pub num_backtests: usize,
    /// Total execution time for all backtests.
    pub duration: Duration,
    /// Collection of `BacktestSummary`s.
    pub summaries: Vec<BacktestSummary<Interval>>,
}

impl<Interval> MultiBacktestSummary<Interval> {
    /// Create a new `MultiBacktestSummary` with the provided data.
    pub fn new<SummaryIter>(duration: Duration, summary_iter: SummaryIter) -> Self
    where
        SummaryIter: IntoIterator<Item = BacktestSummary<Interval>>,
    {
        let summaries = summary_iter.into_iter().collect::<Vec<_>>();

        Self {
            num_backtests: summaries.len(),
            duration,
            summaries,
        }
    }
}

/// Single backtest `TradingSummary` and associated metadata.
#[derive(Debug, PartialEq)]
pub struct BacktestSummary<Interval> {
    /// [`BacktestArgsDynamic`](super::BacktestArgsDynamic) unique identifier that was input for the backtest.
    pub id: SmolStr,
    /// Risk-free return rate used for performance metrics.
    pub risk_free_return: Decimal,
    /// Performance metrics and statistics from the backtest simulated trading.
    pub trading_summary: TradingSummary<Interval>,
}
