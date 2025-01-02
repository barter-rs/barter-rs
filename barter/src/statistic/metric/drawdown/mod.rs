use crate::Timed;
use chrono::{DateTime, TimeDelta, Utc};
use derive_more::Constructor;
use rust_decimal::Decimal;
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
    pub value: Decimal,
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
    pub peak: Option<Decimal>,
    pub drawdown_max: Decimal,
    pub time_peak: Option<DateTime<Utc>>,
    pub time_now: DateTime<Utc>,
}

impl DrawdownGenerator {
    /// Initialise a [`DrawdownGenerator`] from an initial [`Timed`] value.
    pub fn init(point: Timed<Decimal>) -> Self {
        Self {
            peak: Some(point.value),
            drawdown_max: Decimal::ZERO,
            time_peak: Some(point.time),
            time_now: point.time,
        }
    }

    /// Updates the internal [`DrawdownGenerator`] state using the latest [`Timed`] value.
    ///
    /// If the drawdown period has ended (ie/ investment recovers from a trough back above the
    /// previous peak), the functions returns Some(Drawdown), else None is returned.
    pub fn update(&mut self, point: Timed<Decimal>) -> Option<Drawdown> {
        self.time_now = point.time;

        // Handle case of first ever value
        let Some(peak) = self.peak else {
            self.peak = Some(point.value);
            self.time_peak = Some(point.time);
            return None;
        };

        if point.value > peak {
            // Only emit a Drawdown if one actually occurred
            // For example, if we've only ever increased the peak then we don't want to emit
            let ended_drawdown = self.generate();

            // Reset parameters (even if we didn't emit, as we have a new peak)
            self.peak = Some(point.value);
            self.time_peak = Some(point.time);
            self.drawdown_max = Decimal::ZERO;

            ended_drawdown
        } else {
            // Calculate current drawdown at this instant
            let drawdown_current = (peak - point.value).checked_div(peak);

            if let Some(drawdown_current) = drawdown_current {
                // Replace "max drawdown in period" if current drawdown is larger
                if drawdown_current > self.drawdown_max {
                    self.drawdown_max = drawdown_current;
                }
            }

            None
        }
    }

    /// Generates the current [`Drawdown`] at this instant, if it is non-zero.
    pub fn generate(&mut self) -> Option<Drawdown> {
        let time_peak = self.time_peak?;

        (self.drawdown_max != Decimal::ZERO).then_some(Drawdown {
            value: self.drawdown_max,
            time_start: time_peak,
            time_end: self.time_now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::time_plus_days;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use std::str::FromStr;

    #[test]
    fn test_drawdown_generate_update() {
        struct TestCase {
            input: Timed<Decimal>,
            expected_state: DrawdownGenerator,
            expected_output: Option<Drawdown>,
        }

        let time_base = DateTime::<Utc>::MIN_UTC;

        let mut generator = DrawdownGenerator::default();

        let cases = vec![
            // TC0: first ever balance update
            TestCase {
                input: Timed::new(dec!(100.0), time_base),
                expected_state: DrawdownGenerator {
                    peak: Some(dec!(100.0)),
                    drawdown_max: dec!(0.0),
                    time_peak: Some(time_base),
                    time_now: time_base,
                },
                expected_output: None,
            },
            // TC1: peak increases from initial value w/ no drawdown
            TestCase {
                input: Timed::new(dec!(110.0), time_plus_days(time_base, 1)),
                expected_state: DrawdownGenerator {
                    peak: Some(dec!(110.0)),
                    drawdown_max: dec!(0.0),
                    time_peak: Some(time_plus_days(time_base, 1)),
                    time_now: time_plus_days(time_base, 1),
                },
                expected_output: None,
            },
            // TC2: first drawdown occurs
            TestCase {
                input: Timed::new(dec!(99.0), time_plus_days(time_base, 2)),
                expected_state: DrawdownGenerator {
                    peak: Some(dec!(110.0)),
                    drawdown_max: dec!(0.1), // (110-99)/110
                    time_peak: Some(time_plus_days(time_base, 1)),
                    time_now: time_plus_days(time_base, 2),
                },
                expected_output: None,
            },
            // TC3: drawdown increases
            TestCase {
                input: Timed::new(dec!(88.0), time_plus_days(time_base, 3)),
                expected_state: DrawdownGenerator {
                    peak: Some(dec!(110.0)),
                    drawdown_max: dec!(0.2), // (110-88)/110
                    time_peak: Some(time_plus_days(time_base, 1)),
                    time_now: time_plus_days(time_base, 3),
                },
                expected_output: None,
            },
            // TC4: partial recovery (still in drawdown)
            TestCase {
                input: Timed::new(dec!(95.0), time_plus_days(time_base, 4)),
                expected_state: DrawdownGenerator {
                    peak: Some(dec!(110.0)),
                    drawdown_max: dec!(0.2), // max drawdown unchanged
                    time_peak: Some(time_plus_days(time_base, 1)),
                    time_now: time_plus_days(time_base, 4),
                },
                expected_output: None,
            },
            // TC5: full recovery above previous peak - should emit drawdown
            TestCase {
                input: Timed::new(dec!(115.0), time_plus_days(time_base, 5)),
                expected_state: DrawdownGenerator {
                    peak: Some(dec!(115.0)),
                    drawdown_max: dec!(0.0), // reset for new period
                    time_peak: Some(time_plus_days(time_base, 5)),
                    time_now: time_plus_days(time_base, 5),
                },
                expected_output: Some(Drawdown {
                    value: dec!(0.2), // maximum drawdown from previous period
                    time_start: time_plus_days(time_base, 1),
                    time_end: time_plus_days(time_base, 5),
                }),
            },
            // TC6: equal to previous peak (shouldn't trigger new period)
            TestCase {
                input: Timed::new(dec!(115.0), time_plus_days(time_base, 6)),
                expected_state: DrawdownGenerator {
                    peak: Some(dec!(115.0)),
                    drawdown_max: dec!(0.0),
                    time_peak: Some(time_plus_days(time_base, 5)),
                    time_now: time_plus_days(time_base, 6),
                },
                expected_output: None,
            },
            // TC7: tiny drawdown (testing decimal precision)
            TestCase {
                input: Timed::new(
                    Decimal::from_str("114.99999").unwrap(),
                    time_plus_days(time_base, 7),
                ),
                expected_state: DrawdownGenerator {
                    peak: Some(dec!(115.0)),
                    drawdown_max: Decimal::from_str("0.0000000869565217391304347826").unwrap(), // (115-114.99999)/115
                    time_peak: Some(time_plus_days(time_base, 5)),
                    time_now: time_plus_days(time_base, 7),
                },
                expected_output: None,
            },
            // TC8: large peak jump after drawdown
            TestCase {
                input: Timed::new(dec!(200.0), time_plus_days(time_base, 8)),
                expected_state: DrawdownGenerator {
                    peak: Some(dec!(200.0)),
                    drawdown_max: dec!(0.0),
                    time_peak: Some(time_plus_days(time_base, 8)),
                    time_now: time_plus_days(time_base, 8),
                },
                expected_output: Some(Drawdown {
                    value: Decimal::from_str("0.0000000869565217391304347826").unwrap(), // maximum drawdown from previous period
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
