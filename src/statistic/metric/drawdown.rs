use crate::statistic::dispersion::Range;
use chrono::{DateTime, Utc, Duration};
use crate::statistic::summary::trading::{PositionSummariser, calculate_trading_duration};
use crate::portfolio::position::Position;

// Todo: Work out a better way to identify & error handle unclosed Position -> maybe just never
//  handle them? Use a 'closed' boolean flag and throw errors if they are passed?

// Todo: Create DrawdownSummary w/ MaxDrawdown, Durations, Avg Drawdown, etc etc


pub struct MaxDrawdown {
    pub current_equity: EquityPoint,
    pub current_drawdown: Drawdown,
    pub max_drawdown: Drawdown,
}

impl MaxDrawdown {
    pub fn init(starting_equity: f64) -> Self {
        Self {
            current_equity: EquityPoint {
                equity: starting_equity,
                timestamp: Utc::now(),
            },
            current_drawdown: Drawdown::init(starting_equity),
            max_drawdown: Drawdown::init(starting_equity),
        }
    }

    pub fn update(&mut self, position: &Position) {
        // Current equity
        self.current_equity.update(position);

        // Max Drawdown
        match self.current_drawdown.update(&self.current_equity) {
            None => {}
            Some(drawdown) => {
                if drawdown.drawdown > self.max_drawdown.drawdown {
                    self.max_drawdown = drawdown;
                }
            }
        }
    }
}

pub struct EquityPoint {
    pub equity: f64,
    pub timestamp: DateTime<Utc>,
}

impl EquityPoint {
    pub fn update(&mut self, position: &Position) {
        match position.meta.exit_bar_timestamp {
            None => {
                // Position is not exited
                self.equity += position.unreal_profit_loss;
                self.timestamp = position.meta.last_update_timestamp;
            },
            Some(exit_timestamp) => {
                self.equity += position.result_profit_loss;
                self.timestamp = exit_timestamp;
            }
        }
    }
}

pub struct Drawdown {
    pub equity_range: Range,
    pub drawdown: f64,
    pub start_timestamp: DateTime<Utc>,
    pub duration: Duration
}

impl Drawdown {
    pub fn init(starting_equity: f64) -> Self {
        Self {
            equity_range: Range {
                activated: true,
                high: starting_equity,
                low: starting_equity,
            },
            drawdown: 0.0,
            start_timestamp: Utc::now(),
            duration: Duration::zero(),
        }
    }



    pub fn update(&mut self, current: &EquityPoint) -> Option<Drawdown> {
        // Todo: Test condition edge cases!
        // Todo: Add equity_point struct containing equity & time?

        //                                  current_equity >= prev_range_high
        match (self.is_waiting_for_peak(), current.equity >= self.equity_range.high) {

            // A) No current drawdown - waiting for next equity peak (waiting for B)
            (true, true) => {
                self.equity_range.high = current.equity;
                // range low should be None or treated as such at this time
                None
            },

            // B) Start of new drawdown - previous equity point set peak & current equity lower
            (true, false) => {
                self.start_timestamp = current.timestamp;
                self.equity_range.low = current.equity;
                self.drawdown = self.calculate();
                None
            },

            // C) Continuation of drawdown - equity lower than most recent peak
            (false, false) => {
                self.duration = current.timestamp.signed_duration_since(self.start_timestamp);
                self.equity_range.low = current.equity;
                self.drawdown = self.calculate(); // I don't need to calculate this now if I don't want
                None
            },

            // D) End of drawdown - equity has reached new peak (enters A)
            (false, true) => {
                // Todo: This should really be current_equity > prev_range_high, not >=... test & try out alternatives
                // 1 Clone Drawdown from previous iteration to return
                let finished_drawdown = Drawdown {
                    equity_range: self.equity_range.clone(),
                    drawdown: self.drawdown,
                    start_timestamp: self.start_timestamp,
                    duration: self.duration,
                };

                // 2 Clean up - // Todo: ensure other fields such as Duration & timestamp don't need explicitly cleaning up
                self.drawdown = 0.0; // ie/ waiting for peak = true

                // Do I set new range_high now? Or do I wait for next loop
                self.equity_range.high = current.equity;

                Some(finished_drawdown)
            },
        }
    }

    fn is_waiting_for_peak(&self) -> bool {
        self.drawdown == 0.0
    }

    fn calculate(&self) -> f64 {
        // range_low - range_high / range_high
        (-self.equity_range.calculate()) / self.equity_range.high
    }
}



// #[derive(Debug, Clone, PartialOrd, PartialEq)]
// pub struct DrawdownSummary {
//     pub trade_count: usize,
//     pub drawdown_count: usize,
//     pub current_equity: f64,
//     pub equity_range: Range,
//     pub current_drawdown: f64,
//     pub current_drawdown_start_timestamp: DateTime<Utc>,
//     pub current_drawdown_duration: Duration,
//     pub avg_drawdown: f64,
//     pub avg_drawdown_duration: Duration,
//     pub max_drawdown: f64,
//     pub max_drawdown_duration: Duration,
// }
//
// impl DrawdownSummary {
//     pub fn init() -> Self {
//         Self {
//             trade_count: 0,
//             current_equity: 1.0,
//             equity_range: Range {
//                 activated: true,
//                 high: 1.0,
//                 low: 1.0,
//             },
//             current_drawdown: 0.0,
//             current_drawdown_start_timestamp: Utc::now(),
//             current_drawdown_duration: Duration::zero(),
//             avg_drawdown: 0.0,
//             avg_drawdown_duration: Duration::zero(),
//             max_drawdown: 0.0,
//             max_drawdown_duration: Duration::zero(),
//         }
//     }
//
//     pub fn update(&mut self, position: &Position) {
//         // Increment trade counter
//         self.trade_count += 1;
//
//         // Current equity
//         // Todo: Will require to use ratios of since I'm not trading the 100% of my portfolio here...
//         self.current_equity *= (1.0 + position.calculate_profit_loss_return());
//
//         // Drawdown, Start Timestamp & Duration
//         match (self.current_drawdown == 0.0, self.current_equity >= self.equity_range.high) {
//             // Start of new drawdown
//             (true, false) => {
//                 // Todo: Divide by zero error... if current_equity == highest, could change condition from >= -> >
//                 self.current_drawdown = (self.current_equity - self.equity_range.high) / self.equity_range.high;
//                 self.current_drawdown_start_timestamp = position.meta.enter_bar_timestamp;
//                 self.current_drawdown_duration = calculate_trading_duration(&position.meta.enter_bar_timestamp, position);
//             },
//             // Existing drawdown continued
//             (false, false) => {
//                 self.current_drawdown = (self.current_equity - self.equity_range.high) / self.equity_range.high;
//                 self.current_drawdown_duration = calculate_trading_duration(&self.current_drawdown_start_timestamp, position);
//             }
//             // End of existing drawdown
//             (false, true) => {
//                 // Update Average Drawdown & Duration
//                 self.avg_drawdown = WelfordOnline::calculate_mean( // Todo: count needs to be number of drawdowns not trades...
//                                                                    self.avg_drawdown, self.current_drawdown, self.trade_count as f64);
//
//                 let avg_duration_mins = WelfordOnline::calculate_mean(
//                     self.avg_drawdown_duration.num_minutes(),
//                     self.current_drawdown_duration.num_minutes(),
//                     self.trade_count as i64
//                 );
//                 self.avg_drawdown_duration = Duration::minutes(avg_duration_mins);
//
//                 // Update Maximum Drawdown & Duration
//                 if self.current_drawdown > self.max_drawdown {
//                     self.max_drawdown = self.current_drawdown;
//                     self.max_drawdown_duration = self.current_drawdown_duration;
//                 }
//
//                 // Reset Current Drawdown (timestamp & duration overwritten w/ next drawdown)
//                 self.current_drawdown = 0.0;
//             }
//             // No drawdown - ignore
//             _ => {},
//         };
//
//         // Equity Range
//         if self.current_equity >= self.equity_range.high {
//             self.equity_range.high = self.current_equity;
//         }
//         if self.current_equity <= self.equity_range.low {
//             self.equity_range.low = self.current_equity;
//         }
//     }
// }


