use crate::Timed;
use chrono::{DateTime, TimeDelta, Utc};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

pub mod max;
pub mod mean;

/// [`Drawdown`] is the peak-to-trough decline of a value during a specific period. Drawdown is
/// a measure of downside volatility.
///
/// Example use cases are:
///  - Portfolio PnL
///  - Strategy PnL
///  - Instrument PnL
///  - Asset equity
///
/// See documentation: <https://www.investopedia.com/terms/d/drawdown.asp>
#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize, Constructor)]
pub struct Drawdown {
    pub value: f64,
    pub time_start: DateTime<Utc>,
    pub time_end: DateTime<Utc>,
}

impl Drawdown {
    /// Time period of the [`Drawdown`].
    pub fn duration(&self) -> TimeDelta {
        self.time_end.signed_duration_since(self.time_start)
    }
}

/// [`Drawdown`] generator.
///
/// See documentation: <https://www.investopedia.com/terms/d/drawdown.asp>
#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize, Constructor)]
pub struct DrawdownGenerator {
    pub peak: f64,
    pub drawdown_max: f64,
    pub time_peak: DateTime<Utc>,
    pub time_now: DateTime<Utc>,
}

impl DrawdownGenerator {
    /// Initialise a [`DrawdownGenerator`] from an initial [`Timed`] value.
    pub fn init(point: Timed<f64>) -> Self {
        Self {
            peak: point.value,
            drawdown_max: 0.0,
            time_peak: point.time,
            time_now: point.time,
        }
    }

    /// Updates the internal [`DrawdownGenerator`] state using the latest [`Timed`] value.
    ///
    /// If the drawdown period has ended (ie/ investment recovers from a trough back above the
    /// previous peak), the functions returns Some(Drawdown), else None is returned.
    pub fn update(&mut self, point: Timed<f64>) -> Option<Drawdown> {
        self.time_now = point.time;

        if point.value > self.peak {
            // Only emit a Drawdown if one actually occurred
            // For example, if we've only ever increased the peak then we don't want to emit
            let ended_drawdown = self.generate();

            // Reset parameters (even if we didn't emit, as we have a new peak)
            self.peak = point.value;
            self.drawdown_max = 0.0;
            self.time_peak = point.time;

            ended_drawdown
        } else {
            // Calculate current drawdown at this instant
            let drawdown_current = (self.peak - point.value) / self.peak;

            // Replace "max drawdown in period" if current drawdown is larger
            if drawdown_current > self.drawdown_max {
                self.drawdown_max = drawdown_current;
            }

            None
        }
    }

    /// Generates the current [`Drawdown`] at this instant, if it is non-zero.
    pub fn generate(&self) -> Option<Drawdown> {
        (self.drawdown_max != 0.0).then_some(Drawdown {
            value: self.drawdown_max,
            time_start: self.time_peak,
            time_end: self.time_now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::time_plus_days;

    #[test]
    fn test_drawdown_generate_update() {
        struct TestCase {
            input: Timed<f64>,
            expected_state: DrawdownGenerator,
            expected_output: Option<Drawdown>,
        }

        let time_base = DateTime::<Utc>::MIN_UTC;

        let mut generator = DrawdownGenerator {
            peak: 100.0,
            drawdown_max: 0.0,
            time_peak: time_base,
            time_now: time_base,
        };

        let cases = vec![
            // TC0: peak increases from initial value w/ no drawdown
            TestCase {
                input: Timed::new(110.0, time_plus_days(time_base, 1)),
                expected_state: DrawdownGenerator {
                    peak: 110.0,
                    drawdown_max: 0.0,
                    time_peak: time_plus_days(time_base, 1),
                    time_now: time_plus_days(time_base, 1),
                },
                expected_output: None,
            },
            // TC1: first drawdown occurs
            TestCase {
                input: Timed::new(99.0, time_plus_days(time_base, 2)),
                expected_state: DrawdownGenerator {
                    peak: 110.0,
                    drawdown_max: 0.1, // (110-99)/110
                    time_peak: time_plus_days(time_base, 1),
                    time_now: time_plus_days(time_base, 2),
                },
                expected_output: None,
            },
            // TC2: drawdown increases
            TestCase {
                input: Timed::new(88.0, time_plus_days(time_base, 3)),
                expected_state: DrawdownGenerator {
                    peak: 110.0,
                    drawdown_max: 0.2, // (110-88)/110
                    time_peak: time_plus_days(time_base, 1),
                    time_now: time_plus_days(time_base, 3),
                },
                expected_output: None,
            },
            // TC3: partial recovery (still in drawdown)
            TestCase {
                input: Timed::new(95.0, time_plus_days(time_base, 4)),
                expected_state: DrawdownGenerator {
                    peak: 110.0,
                    drawdown_max: 0.2, // max drawdown unchanged
                    time_peak: time_plus_days(time_base, 1),
                    time_now: time_plus_days(time_base, 4),
                },
                expected_output: None,
            },
            // TC4: full recovery above previous peak - should emit drawdown
            TestCase {
                input: Timed::new(115.0, time_plus_days(time_base, 5)),
                expected_state: DrawdownGenerator {
                    peak: 115.0,
                    drawdown_max: 0.0, // reset for new period
                    time_peak: time_plus_days(time_base, 5),
                    time_now: time_plus_days(time_base, 5),
                },
                expected_output: Some(Drawdown {
                    value: 0.2, // maximum drawdown from previous period
                    time_start: time_plus_days(time_base, 1),
                    time_end: time_plus_days(time_base, 5),
                }),
            },
            // TC5: equal to previous peak (shouldn't trigger new period)
            TestCase {
                input: Timed::new(115.0, time_plus_days(time_base, 6)),
                expected_state: DrawdownGenerator {
                    peak: 115.0,
                    drawdown_max: 0.0,
                    time_peak: time_plus_days(time_base, 5),
                    time_now: time_plus_days(time_base, 6),
                },
                expected_output: None,
            },
            // TC6: tiny drawdown (testing floating point precision)
            TestCase {
                input: Timed::new(114.99999, time_plus_days(time_base, 7)),
                expected_state: DrawdownGenerator {
                    peak: 115.0,
                    drawdown_max: 8.695652176673163e-8, // (115-114.99999)/115
                    time_peak: time_plus_days(time_base, 5),
                    time_now: time_plus_days(time_base, 7),
                },
                expected_output: None,
            },
            // TC7: large peak jump after drawdown
            TestCase {
                input: Timed::new(200.0, time_plus_days(time_base, 8)),
                expected_state: DrawdownGenerator {
                    peak: 200.0,
                    drawdown_max: 0.0,
                    time_peak: time_plus_days(time_base, 8),
                    time_now: time_plus_days(time_base, 8),
                },
                expected_output: Some(Drawdown {
                    value: 8.695652176673163e-8, // maximum drawdown from previous period
                    time_start: time_plus_days(time_base, 5),
                    time_end: time_plus_days(time_base, 8),
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let output = generator.update(test.input);
            assert_eq!(generator, test.expected_state, "TC{index} failed");
            assert_eq!(output, test.expected_output, "TC{index} failed");
        }
    }
}
