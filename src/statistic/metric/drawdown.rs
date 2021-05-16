use crate::statistic::dispersion::Range;
use chrono::{DateTime, Utc, Duration};
use crate::portfolio::position::Position;
use crate::statistic::algorithm::WelfordOnline;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Drawdown {
    pub trade_count: usize,
    pub current_equity: f64,
    pub equity_range: Range,
    pub current_drawdown: f64,
    pub current_drawdown_start_timestamp: DateTime<Utc>,
    pub current_drawdown_duration: Duration,
    pub avg_drawdown: f64,
    pub avg_drawdown_duration: Duration,
    pub max_drawdown: f64,
    pub max_drawdown_duration: Duration,
}


impl Drawdown {
    pub fn init() -> Self {
        Self {
            trade_count: 0,
            current_equity: 1.0,
            equity_range: Range {
                activated: true,
                highest: 1.0,
                lowest: 1.0,
            },
            current_drawdown: 0.0,
            current_drawdown_start_timestamp: Utc::now(),
            current_drawdown_duration: Duration::zero(),
            avg_drawdown: 0.0,
            avg_drawdown_duration: Duration::zero(),
            max_drawdown: 0.0,
            max_drawdown_duration: Duration::zero(),
        }
    }

    pub fn update(&mut self, position: &Position) {
        // Increment trade counter
        self.trade_count += 1;

        // Current equity
        // Todo: Will require to use ratios of since I'm not trading the 100% of my portfolio here...
        self.current_equity *= (1.0 + position.calculate_profit_loss_return());

        // Drawdown, Start Timestamp & Duration
        match (self.current_drawdown == 0.0, self.current_equity >= self.equity_range.highest) {
            // Start of new drawdown
            (true, false) => {
                // Todo: Divide by zero error... if current_equity == highest, could change condition from >= -> >
                self.current_drawdown = (self.current_equity - self.equity_range.highest) / self.equity_range.highest;
                self.current_drawdown_start_timestamp = position.meta.enter_bar_timestamp;
                self.current_drawdown_duration = calculate_trading_duration(&position.meta.enter_bar_timestamp, position);
            },
            // Existing drawdown continued
            (false, false) => {
                self.current_drawdown = (self.current_equity - self.equity_range.highest) / self.equity_range.highest;
                self.current_drawdown_duration = calculate_trading_duration(&self.current_drawdown_start_timestamp, position);
            }
            // End of existing drawdown
            (false, true) => {
                // Update Average Drawdown & Duration
                self.avg_drawdown = WelfordOnline::calculate_mean(
                    self.avg_drawdown, self.current_drawdown, self.trade_count as f64);

                let avg_duration_mins = WelfordOnline::calculate_mean(
                    self.avg_drawdown_duration.num_minutes(),
                    self.current_drawdown_duration.num_minutes(),
                    self.trade_count as i64
                );
                self.avg_drawdown_duration = Duration::minutes(avg_duration_mins);

                // Update Maximum Drawdown & Duration
                if self.current_drawdown > self.max_drawdown {
                    self.max_drawdown = self.current_drawdown;
                    self.max_drawdown_duration = self.current_drawdown_duration;
                }

                // Reset Current Drawdown (timestamp & duration overwritten w/ next drawdown)
                self.current_drawdown = 0.0;
            }
            // No drawdown - ignore
            _ => {},
        };

        // Equity Range
        if self.current_equity >= self.equity_range.highest {
            self.equity_range.highest = self.current_equity;
        }
        if self.current_equity <= self.equity_range.lowest {
            self.equity_range.lowest = self.current_equity;
        }
    }
}

// Todo: Find a home for this function
fn calculate_trading_duration(start_timestamp: &DateTime<Utc>, position: &Position) -> Duration {
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