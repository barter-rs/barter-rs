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
            self.pnl_returns.downside_deviation(),
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

        let win_rate = WinRate::calculate(
            self.pnl_returns.total.count - self.pnl_returns.losses.count,
            self.pnl_returns.total.count,
        );

        let profit_factor = ProfitFactor::calculate(
            self.pnl_returns.total.sum - self.pnl_returns.losses.sum,
            self.pnl_returns.losses.sum,
        );

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{statistic::time::Daily, test_utils::time_plus_days};
    use barter_execution::trade::{AssetFees, TradeId};
    use barter_instrument::{Side, asset::QuoteAsset, instrument::name::InstrumentNameInternal};
    use rust_decimal::MathematicalOps;
    use rust_decimal_macros::dec;

    fn exited(
        pnl_realised: Decimal,
        day: u64,
    ) -> PositionExited<QuoteAsset, InstrumentNameInternal> {
        let base_time = DateTime::<Utc>::MIN_UTC;
        PositionExited {
            instrument: InstrumentNameInternal::new("instrument"),
            side: Side::Buy,
            price_entry_average: dec!(100.0),
            quantity_abs_max: dec!(1.0),
            pnl_realised,
            fees_enter: AssetFees {
                asset: QuoteAsset,
                fees: dec!(0.0),
            },
            fees_exit: AssetFees {
                asset: QuoteAsset,
                fees: dec!(0.0),
            },
            time_enter: time_plus_days(base_time, day),
            time_exit: time_plus_days(base_time, day + 1),
            trades: vec![TradeId::new("trade_id")],
        }
    }

    fn tear_sheet(pnls: &[Decimal]) -> TearSheet<Daily> {
        let mut generator = TearSheetGenerator::init(DateTime::<Utc>::MIN_UTC);
        for (day, pnl) in pnls.iter().enumerate() {
            generator.update_from_position(&exited(*pnl, day as u64));
        }
        generator.generate(Decimal::ZERO, Daily)
    }

    /// 7 wins of +10 and 3 losses of -5 on a notional of 100.
    fn seven_wins_three_losses() -> Vec<Decimal> {
        let mut pnls = vec![dec!(10.0); 7];
        pnls.extend([dec!(-5.0); 3]);
        pnls
    }

    #[test]
    fn test_tear_sheet_win_rate_counts_wins() {
        let tear = tear_sheet(&seven_wins_three_losses());
        let win_rate = tear.win_rate.unwrap().value;
        assert_eq!(
            win_rate,
            dec!(0.7),
            "7 of 10 positions won, win_rate reported {win_rate}"
        );
    }

    #[test]
    fn test_tear_sheet_profit_factor_is_gross_profit_over_gross_loss() {
        let tear = tear_sheet(&seven_wins_three_losses());
        let profit_factor = tear.profit_factor.unwrap().value;
        // gross profit 0.70 / gross loss 0.15
        let expected = dec!(0.70) / dec!(0.15);
        assert!(
            (profit_factor - expected).abs() < dec!(0.0001),
            "expected {expected}, profit_factor reported {profit_factor}"
        );
    }

    #[test]
    fn test_tear_sheet_profit_factor_of_only_losses_is_worst() {
        // ProfitFactor documents Decimal::MIN for losses with no profits.
        let tear = tear_sheet(&[dec!(-5.0), dec!(-5.0), dec!(-5.0)]);
        let profit_factor = tear.profit_factor.map(|pf| pf.value);
        assert_eq!(
            profit_factor,
            Some(Decimal::MIN),
            "no winning positions, profit_factor reported {profit_factor:?}"
        );
    }

    #[test]
    fn test_tear_sheet_sortino_uses_downside_deviation_over_all_positions() {
        let tear = tear_sheet(&seven_wins_three_losses());
        let sortino = tear.sortino_ratio.value;
        // Downside deviation about a zero target over all 10 returns: sqrt(3 * 0.05^2 / 10).
        // The losses are equal, so their own standard deviation is zero.
        let downside_deviation = (dec!(3) * dec!(0.0025) / dec!(10)).sqrt().unwrap();
        let mean = dec!(0.055);
        // 10 days of trading, scaled to Daily.
        let expected = mean / downside_deviation * (dec!(1) / dec!(10)).sqrt().unwrap();
        assert!(
            (sortino - expected).abs() < dec!(0.0001),
            "expected {expected}, sortino reported {sortino}"
        );
    }
}
