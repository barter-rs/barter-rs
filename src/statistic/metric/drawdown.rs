use crate::statistic::dispersion::Range;
use chrono::{DateTime, Utc, Duration};
use crate::statistic::summary::trading::{PositionSummariser, calculate_trading_duration};
use crate::portfolio::position::Position;

// Todo: Work out a better way to identify & error handle unclosed Position -> maybe just never
//  handle them? Use a 'closed' boolean flag and throw errors if they are passed?

// Todo: Create DrawdownSummary w/ MaxDrawdown, Durations, Avg Drawdown, etc etc

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct MaxDrawdown {
    pub drawdown: Drawdown,
}

impl MaxDrawdown {
    pub fn init() -> Self {
        Self {
            drawdown: Drawdown::default(),
        }
    }

    pub fn update(&mut self, drawdown: Drawdown) {
        if drawdown.drawdown > self.drawdown.drawdown {
            self.drawdown = drawdown;
        }
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Drawdown {
    pub equity_range: Range,
    pub drawdown: f64,
    pub start_timestamp: DateTime<Utc>,
    pub duration: Duration
}

impl Default for Drawdown {
    fn default() -> Self {
        Self {
            equity_range: Default::default(),
            drawdown: 0.0,
            start_timestamp: Utc::now(),
            duration: Duration::zero(),
        }
    }
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
        match (self.is_waiting_for_peak(), current.equity > self.equity_range.high) {

            // A) No current drawdown - waiting for next equity peak (waiting for B)
            (true, true) => {
                self.equity_range.high = current.equity;
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
                self.equity_range.update(current.equity);
                self.drawdown = self.calculate(); // I don't need to calculate this now if I don't want
                None
            },

            // D) End of drawdown - equity has reached new peak (enters A)
            (false, true) => {
                // Clone Drawdown from previous iteration to return
                let finished_drawdown = Drawdown {
                    equity_range: self.equity_range.clone(),
                    drawdown: self.drawdown,
                    start_timestamp: self.start_timestamp,
                    duration: self.duration,
                };

                // Clean up - start_timestamp overwritten next drawdown start
                self.drawdown = 0.0; // ie/ waiting for peak = true
                self.duration = Duration::zero();

                // Set new equity peak in preparation for next iteration
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

#[derive(Debug, Clone, PartialOrd, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Add;

    fn equity_update_position_closed(exit_timestamp: DateTime<Utc>, result_pnl: f64) -> Position {
        let mut position = Position::default();
        position.meta.exit_bar_timestamp = Some(exit_timestamp);
        position.result_profit_loss = result_pnl;
        position
    }

    fn equity_update_position_open(last_update_timestamp: DateTime<Utc>, unreal_pnl: f64) -> Position {
        let mut position = Position::default();
        position.meta.last_update_timestamp = last_update_timestamp;
        position.unreal_profit_loss = unreal_pnl;
        position
    }

    #[test]
    fn equity_point_update() {
        struct TestCase {
            position: Position,
            expected_equity: f64,
            expected_timestamp: DateTime<Utc>,
        }

        let base_timestamp = Utc::now();

        let mut equity_point = EquityPoint {
            equity: 100.0,
            timestamp: base_timestamp
        };

        let test_cases = vec![
            TestCase {
                position: equity_update_position_closed(base_timestamp.add(Duration::days(1)), 10.0),
                expected_equity: 110.0, expected_timestamp: base_timestamp.add(Duration::days(1))
            },
            TestCase {
                position: equity_update_position_open(base_timestamp.add(Duration::days(2)), -10.0),
                expected_equity: 100.0, expected_timestamp: base_timestamp.add(Duration::days(2))
            },
            TestCase {
                position: equity_update_position_closed(base_timestamp.add(Duration::days(3)), -55.9),
                expected_equity: 44.1, expected_timestamp: base_timestamp.add(Duration::days(3))
            },
            TestCase {
                position: equity_update_position_open(base_timestamp.add(Duration::days(4)), 68.7),
                expected_equity: 112.8, expected_timestamp: base_timestamp.add(Duration::days(4))
            },
            TestCase {
                position: equity_update_position_closed(base_timestamp.add(Duration::days(5)), 99999.0),
                expected_equity: 100111.8, expected_timestamp: base_timestamp.add(Duration::days(5))
            },
            TestCase {
                position: equity_update_position_open(base_timestamp.add(Duration::days(5)), 0.2),
                expected_equity: 100112.0, expected_timestamp: base_timestamp.add(Duration::days(5))
            },
        ];

        for test in test_cases {
            equity_point.update(&test.position);
            let equity_diff = equity_point.equity - test.expected_equity;
            assert!(equity_diff < 1e-10);
            assert_eq!(equity_point.timestamp, test.expected_timestamp)
        }
    }

    #[test]
    fn drawdown_update() {
        struct TestCase {
            input_equity: EquityPoint,
            expected_drawdown: Drawdown,
        }
        let base_timestamp = Utc::now();
        let starting_equity = 100.0;

        let mut drawdown = Drawdown {
            equity_range: Range {
                activated: true,
                high: starting_equity,
                low: starting_equity,
            },
            drawdown: 0.0,
            start_timestamp: base_timestamp,
            duration: Duration::zero(),
        };

        let test_cases = vec![
            TestCase { // Test case 0: No current drawdown
                input_equity: EquityPoint { equity: 110.0, timestamp: base_timestamp.add(Duration::days(1)) },
                expected_drawdown: Drawdown {
                    equity_range: Range { activated: true, high: 110.0, low: 100.0 },
                    drawdown: 0.0,
                    start_timestamp: base_timestamp,
                    duration: Duration::zero(),
                }
            },
            TestCase { // Test case 1: Start of new drawdown w/ lower equity than peak
                input_equity: EquityPoint { equity: 100.0, timestamp: base_timestamp.add(Duration::days(2)) },
                expected_drawdown: Drawdown {
                    equity_range: Range { activated: true, high: 110.0, low: 100.0 },
                    drawdown: (-10.0/110.0),
                    start_timestamp: base_timestamp.add(Duration::days(2)),
                    duration: Duration::zero(),
                }
            },
            TestCase { // Test case 2: Continuation of drawdown w/ lower equity than previous
                input_equity: EquityPoint { equity: 90.0, timestamp: base_timestamp.add(Duration::days(3)) },
                expected_drawdown: Drawdown {
                    equity_range: Range { activated: true, high: 110.0, low: 90.0 },
                    drawdown: (-20.0/110.0),
                    start_timestamp: base_timestamp.add(Duration::days(2)),
                    duration: Duration::days(1),
                }
            },
            TestCase { // Test case 3: Continuation of drawdown w/ higher equity than previous but not higher than peak
                input_equity: EquityPoint { equity: 95.0, timestamp: base_timestamp.add(Duration::days(4)) },
                expected_drawdown: Drawdown {
                    equity_range: Range { activated: true, high: 110.0, low: 90.0 },
                    drawdown: (-20.0/110.0),
                    start_timestamp: base_timestamp.add(Duration::days(2)),
                    duration: Duration::days(2),
                }
            },
            TestCase { // Test case 4: End of drawdown w/ equity higher than peak
                input_equity: EquityPoint { equity: 120.0, timestamp: base_timestamp.add(Duration::days(5)) },
                expected_drawdown: Drawdown {
                    equity_range: Range { activated: true, high: 120.0, low: 90.0 },
                    drawdown: 0.0,
                    start_timestamp: base_timestamp.add(Duration::days(2)),
                    duration: Duration::zero(),
                }
            },
            TestCase { // Test case 5: No current drawdown w/ residual start_timestamp from previous
                input_equity: EquityPoint { equity: 200.0, timestamp: base_timestamp.add(Duration::days(6)) },
                expected_drawdown: Drawdown {
                    equity_range: Range { activated: true, high: 200.0, low: 90.0 },
                    drawdown: 0.0,
                    start_timestamp: base_timestamp.add(Duration::days(2)),
                    duration: Duration::zero(),
                }
            },
            TestCase { // Test case 6: Start of new drawdown w/ lower equity than peak & residual fields from previous drawdown
                input_equity: EquityPoint { equity: 180.0, timestamp: base_timestamp.add(Duration::days(7)) },
                expected_drawdown: Drawdown {
                    equity_range: Range { activated: true, high: 200.0, low: 180.0 },
                    drawdown: (-20.0/200.0),
                    start_timestamp: base_timestamp.add(Duration::days(7)),
                    duration: Duration::zero(),
                }
            },
            TestCase { // Test case 7: Continuation of drawdown w/ equity equal to peak
                input_equity: EquityPoint { equity: 200.0, timestamp: base_timestamp.add(Duration::days(8)) },
                expected_drawdown: Drawdown {
                    equity_range: Range { activated: true, high: 200.0, low: 180.0 },
                    drawdown: (-20.0/200.0),
                    start_timestamp: base_timestamp.add(Duration::days(7)),
                    duration: Duration::days(1),
                }
            },
            TestCase { // Test case 8: End of drawdown w/ equity higher than peak
                input_equity: EquityPoint { equity: 200.01, timestamp: base_timestamp.add(Duration::days(9)) },
                expected_drawdown: Drawdown {
                    equity_range: Range { activated: true, high: 200.01, low: 180.0 },
                    drawdown: 0.0,
                    start_timestamp: base_timestamp.add(Duration::days(7)),
                    duration: Duration::zero(),
                }
            },
        ];

        for (index, test) in test_cases.into_iter().enumerate() {
            drawdown.update(&test.input_equity);
            assert_eq!(drawdown, test.expected_drawdown, "Input: {:?}", index)
        }
    }
}