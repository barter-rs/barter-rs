use crate::{
    Timed,
    engine::state::position::PositionExited,
    statistic::{
        metric::{
            calmar::CalmarRatio,
            drawdown::{
                Drawdown, DrawdownGenerator,
                max::{MaxDrawdown, MaxDrawdownGenerator},
                mean::{MeanDrawdown, MeanDrawdownGenerator},
            },
            profit_factor::ProfitFactor,
            rate_of_return::RateOfReturn,
            sharpe::SharpeRatio,
            sortino::SortinoRatio,
            win_rate::WinRate,
        },
        summary::pnl::PnLReturns,
        time::TimeInterval,
    },
};
use chrono::{DateTime, TimeDelta, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// TearSheet summarising the trading performance related to an instrument.
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct TearSheet<Interval> {
    pub pnl: Decimal,
    pub pnl_return: RateOfReturn<Interval>,
    pub sharpe_ratio: SharpeRatio<Interval>,
    pub sortino_ratio: SortinoRatio<Interval>,
    pub calmar_ratio: CalmarRatio<Interval>,
    pub pnl_drawdown: Option<Drawdown>,
    pub pnl_drawdown_mean: Option<MeanDrawdown>,
    pub pnl_drawdown_max: Option<MaxDrawdown>,
    pub win_rate: Option<WinRate>,
    pub profit_factor: Option<ProfitFactor>,
}

/// Generator for a [`TearSheet`].
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct TearSheetGenerator {
    /// Trading session start time defined by the [`Engine`](crate::engine::Engine) clock.
    pub time_engine_start: DateTime<Utc>,

    /// Trading session end time defined by the [`Engine`](crate::engine::Engine) clock.
    pub time_engine_now: DateTime<Utc>,

    pub pnl_returns: PnLReturns,
    pub pnl_drawdown: DrawdownGenerator,
    pub pnl_drawdown_mean: MeanDrawdownGenerator,
    pub pnl_drawdown_max: MaxDrawdownGenerator,
}

impl TearSheetGenerator {
    /// Initialise a [`TearSheetGenerator`] with an initial timestamp.
    pub fn init(time_engine_start: DateTime<Utc>) -> Self {
        Self {
            time_engine_start,
            time_engine_now: time_engine_start,
            pnl_returns: PnLReturns::default(),
            pnl_drawdown: DrawdownGenerator::default(),
            pnl_drawdown_mean: MeanDrawdownGenerator::default(),
            pnl_drawdown_max: MaxDrawdownGenerator::default(),
        }
    }

    /// Update the [`TearSheetGenerator`] from the next [`PositionExited`].
    pub fn update_from_position<AssetKey, InstrumentKey>(
        &mut self,
        position: &PositionExited<AssetKey, InstrumentKey>,
    ) {
        self.time_engine_now = position.time_exit;
        self.pnl_returns.update(position);

        if let Some(next_drawdown) = self
            .pnl_drawdown
            .update(Timed::new(self.pnl_returns.pnl_raw, self.time_engine_now))
        {
            self.pnl_drawdown_mean.update(&next_drawdown);
            self.pnl_drawdown_max.update(&next_drawdown);
        }
    }

    /// Generate the latest [`TearSheet`] at the specific [`TimeInterval`].
    ///
    /// For example, pass [`Annual365`](super::super::time::Annual365) to generate a crypto-centric
    /// (24/7 trading) annualised [`TearSheet`].
    pub fn generate<Interval>(
        &mut self,
        risk_free_return: Decimal,
        interval: Interval,
    ) -> TearSheet<Interval>
    where
        Interval: TimeInterval,
    {
        let trading_period = self
            .time_engine_now
            .signed_duration_since(self.time_engine_start)
            .max(TimeDelta::seconds(1));

        let sharpe_ratio = SharpeRatio::calculate(
            risk_free_return,
            self.pnl_returns.total.mean,
            self.pnl_returns.total.dispersion.std_dev,
            trading_period,
        )
        .scale(interval);

        let sortino_ratio = SortinoRatio::calculate(
            risk_free_return,
            self.pnl_returns.total.mean,
            self.pnl_returns.losses.dispersion.std_dev,
            trading_period,
        )
        .scale(interval);

        let current_pnl_drawdown = self.pnl_drawdown.generate();
        if let Some(current_pnl_drawdown) = &current_pnl_drawdown {
            self.pnl_drawdown_mean.update(current_pnl_drawdown);
            self.pnl_drawdown_max.update(current_pnl_drawdown);
        }
        let pnl_drawdown_mean = self.pnl_drawdown_mean.generate();
        let pnl_drawdown_max = self.pnl_drawdown_max.generate();

        let calmar_ratio = CalmarRatio::calculate(
            risk_free_return,
            self.pnl_returns.total.mean,
            // Zero drawdown risk handled by CalmarRatio::calculate
            pnl_drawdown_max
                .as_ref()
                .unwrap_or(&MaxDrawdown(Drawdown::default()))
                .0
                .value,
            trading_period,
        )
        .scale(interval);

        let pnl_return =
            RateOfReturn::calculate(self.pnl_returns.total.mean, trading_period).scale(interval);

        let win_rate =
            WinRate::calculate(self.pnl_returns.losses.count, self.pnl_returns.total.count);

        let profit_factor =
            ProfitFactor::calculate(self.pnl_returns.total.sum, self.pnl_returns.losses.sum);

        TearSheet {
            sharpe_ratio,
            sortino_ratio,
            calmar_ratio,
            pnl: self.pnl_returns.pnl_raw,
            pnl_return,
            pnl_drawdown: current_pnl_drawdown,
            pnl_drawdown_mean,
            pnl_drawdown_max,
            win_rate,
            profit_factor,
        }
    }

    /// Reset the internal state, using a new starting `DateTime<Utc>` as seed.
    pub fn reset(&mut self, time_engine_start: DateTime<Utc>) {
        *self = Self::init(time_engine_start);
    }
}
