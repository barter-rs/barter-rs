use crate::statistic::sharpe_ratio_spike::{PnLReturnView, DataSummary};
use chrono::Duration;
use crate::portfolio::position::Position;
use crate::statistic::dispersion::Range;

pub trait NewMetricRolling {
    const METRIC_ID: &'static str;
    fn init() -> Self;
    fn update(&mut self, pnl_returns: &PnLReturnView);
}

pub struct SharpeRatio {
    pub trading_duration: Duration,
    pub trade_count: usize,
    pub risk_free_return: f64,
    pub sharpe_ratio_per_trade: f64,
}

impl SharpeRatio {
    fn init(risk_free_return: f64) -> Self {
        Self {
            trading_duration: Duration::zero(),
            trade_count: 0,
            risk_free_return,
            sharpe_ratio_per_trade: 0.0
        }
    }

    fn update(&mut self, pnl_returns: &PnLReturnView) {
        // Update Trade Duration & Counter
        self.trading_duration = pnl_returns.duration;
        self.trade_count = pnl_returns.total.count;

        // Calculate Sharpe Ratio Per Trade
        self.sharpe_ratio_per_trade = (pnl_returns.total.mean - self.risk_free_return)
            / pnl_returns.total.dispersion.standard_deviation
    }

    fn calculate_daily(&self) -> f64 {
        let trades_per_day = self.trade_count / self.trading_duration.num_days() as usize;
        self.sharpe_ratio_per_trade * trades_per_day as f64.sqrt()
    }

    fn calculate_annual(&self, trading_days: usize) -> f64 {
        self.calculate_daily() * trading_days as f64.sqrt()
    }
}

pub struct SortinoRatio {
    pub trading_duration: Duration,
    pub trade_count: usize,
    pub risk_free_return: f64,
    pub sortino_rate_per_trade: f64,
}

impl SortinoRatio {
    fn init(risk_free_return: f64) -> Self {
        Self {
            trading_duration: Duration::zero(),
            trade_count: 0,
            risk_free_return,
            sortino_rate_per_trade: 0.0
        }
    }

    fn update(&mut self, pnl_returns: &PnLReturnView) {
        // Update Trade Duration & Counter
        self.trading_duration = pnl_returns.duration;
        self.trade_count = pnl_returns.total.count;

        // Calculate Sortino Ratio Per Trade
        self.sortino_rate_per_trade = (pnl_returns.total.mean - self.risk_free_return)
            / pnl_returns.losses.dispersion.standard_deviation
    }

    fn calculate_daily(&self) -> f64 {
        let trades_per_day = self.trade_count / self.trading_duration.num_days() as usize;
        self.sortino_rate_per_trade * trades_per_day as f64.sqrt()
    }

    fn calculate_annual(&self, trading_days: usize) -> f64 {
        self.calculate_daily() * trading_days as f64.sqrt()
    }
}


// Todo:
//  - Split out drawdown into several metrics, ie/ MaxDrawdown, MaxDrawdown duration,
//    Drawdown avg, Drawdown avg duration.
//  - Let Drawdown take returns instead and have starting_equity = 1.0 (normalised) -> current_equity *= position.pnl_return

pub struct Drawdown {
    pub starting_equity: f64,
    pub current_equity: f64,
    pub current_drawdown: f64,
    pub equity_range: Range,
    pub avg_drawdown: f64,
    pub avg_drawdown_duration: Duration,
    pub max_drawdown: f64,
    pub max_drawdown_duration: Duration,
}


impl Drawdown {
    fn init(starting_equity: f64) -> Self {
        Self {
            starting_equity,
            current_equity: starting_equity,
            current_drawdown: 0.0,
            equity_range: Range {
                highest: starting_equity,
                lowest: starting_equity,
            },
            avg_drawdown: 0.0,
            avg_drawdown_duration: Duration,
            max_drawdown: 0.0,
            max_drawdown_duration: Duration::zero()
        }
    }

    fn update(&mut self, position: &Position) {
        // Current equity
        self.current_equity += position.result_profit_loss;

        // Current Drawdown
        self.current_drawdown = match self.current_equity >= self.equity_range.highest {
            true => 0.0,
            false => (self.current_equity - self.equity_range.highest) / self.equity_range.highest,
        };


        // Equity Range
        if self.current_equity >= self.equity_range.highest {
            self.equity_range.highest = self.current_equity;
        }
        if self.current_equity <= self.equity_range.lowest {
            self.equity_range.lowest = self.current_equity;
        }

        // Current Drawdown
        let prev_highest



    }
}




















