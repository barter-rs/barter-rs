use std::collections::HashMap;
use crate::execution::fill::FillEvent;
use crate::portfolio::position::{Position, Direction};
use crate::portfolio::error::PortfolioError;

pub struct RollingStatistic {
    profit_loss: HashMap<String, ProfitLoss>,
    // sharpe_ratio: HashMap<String, f64>,
}

impl RollingStatistic {
    pub fn new() -> Self {
        Self {
            profit_loss: HashMap::new(),
        }
    }

    pub fn update_from_fill_exit(&mut self, position: &Position) {

        if self.profit_loss.is_empty() {

            self.profit_loss.insert(fill.symbol.clone(), ProfitLoss::init())

        }
    }
}

struct ProfitLoss {
    long_count: f64,
    short_count: f64,
    avg_long_pnl_pct: f64,
    avg_short_pnl_pct: f64,
    avg_pnl_pct: f64,
}

impl ProfitLoss {
    pub fn init(position: &Position) -> Self {
        let (long_count, long_pnl, short_count, short_pnl) = match position.direction {
            Direction::Long => (1.0, position.result_profit_loss, 0.0, 0.0),
            Direction::Short => (0.0, 0.0, 1.0, position.result_profit_loss)
        };

        ProfitLoss::builder()
            .long_count(long_count)
            .short_count(short_count)
            .avg_long_pnl_pct(long_pnl)
            .avg_short_pnl_pct(short_pnl)
            .avg_pnl_pct(position.result_profit_loss)
            .build()
            .unwrap()
    }

    pub fn builder() -> ProfitLossBuilder {
        ProfitLossBuilder::new()
    }
}

pub struct ProfitLossBuilder {
    long_count: Option<f64>,
    short_count: Option<f64>,
    avg_long_pnl_pct: Option<f64>,
    avg_short_pnl_pct: Option<f64>,
    avg_pnl_pct: Option<f64>,
}

impl ProfitLossBuilder {
    pub fn new() -> Self {
        Self {
            long_count: None,
            short_count: None,
            avg_long_pnl_pct: None,
            avg_short_pnl_pct: None,
            avg_pnl_pct: None,
        }
    }

    pub fn long_count(mut self, value: f64) -> Self {
        self.long_count = Some(value);
        self
    }

    pub fn short_count(mut self, value: f64) -> Self {
        self.short_count = Some(value);
        self
    }

    pub fn avg_long_pnl_pct(mut self, value: f64) -> Self {
        self.avg_long_pnl_pct = Some(value);
        self
    }

    pub fn avg_short_pnl_pct(mut self, value: f64) -> Self {
        self.avg_short_pnl_pct = Some(value);
        self
    }

    pub fn avg_pnl_pct(mut self, value: f64) -> Self {
        self.avg_pnl_pct = Some(value);
        self
    }

    pub fn build(self) -> Result<ProfitLoss, PortfolioError> {
        if let (
            Some(long_count),
            Some(short_count),
            Some(avg_long_pnl_pct),
            Some(avg_short_pnl_pct),
            Some(avg_pnl_pct),
        ) = (
            self.long_count,
            self.short_count,
            self.avg_long_pnl_pct,
            self.avg_short_pnl_pct,
            self.avg_pnl_pct,
        ) {
            Ok(ProfitLoss {
                long_count,
                short_count,
                avg_long_pnl_pct,
                avg_short_pnl_pct,
                avg_pnl_pct,
            })
        } else {
            Err(PortfolioError::BuilderIncomplete)
        }
    }
}