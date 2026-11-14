use crate::statistic::{algorithm::welford_online, metric::drawdown::Drawdown};
use derive_more::Constructor;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// [`MeanDrawdown`] is defined as the mean (average) drawdown value and millisecond duration from
/// a collection of [`Drawdown`]s.
#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize, Constructor)]
pub struct MeanDrawdown {
    pub mean_drawdown: Decimal,
    pub mean_drawdown_ms: i64,
}

/// [`MeanDrawdown`] generator.
#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize, Constructor)]
pub struct MeanDrawdownGenerator {
    pub count: u64,
    pub mean_drawdown: Option<MeanDrawdown>,
}

impl MeanDrawdownGenerator {
    /// Initialise a [`MeanDrawdownGenerator`] from an initial [`Drawdown`].
    pub fn init(drawdown: Drawdown) -> Self {
        Self {
            count: 1,
            mean_drawdown: Some(MeanDrawdown {
                mean_drawdown: drawdown.value,
                mean_drawdown_ms: drawdown.duration().num_milliseconds(),
            }),
        }
    }

    /// Updates the mean drawdown and mean drawdown duration using the next [`Drawdown`] provided.
    pub fn update(&mut self, next_drawdown: &Drawdown) {
        self.count += 1;

        let mean_drawdown = match self.mean_drawdown.take() {
            Some(MeanDrawdown {
                mean_drawdown,
                mean_drawdown_ms,
            }) => MeanDrawdown {
                mean_drawdown: welford_online::calculate_mean(
                    mean_drawdown,
                    next_drawdown.value,
                    Decimal::from(self.count),
                ),
                mean_drawdown_ms: welford_online::calculate_mean(
                    mean_drawdown_ms,
                    next_drawdown.duration().num_milliseconds(),
                    self.count as i64,
                ),
            },
            None => MeanDrawdown {
                mean_drawdown: next_drawdown.value,
                mean_drawdown_ms: next_drawdown.duration().num_milliseconds(),
            },
        };

        self.mean_drawdown = Some(mean_drawdown)
    }

    /// Generate the current [`MeanDrawdown`], if one exists.
    pub fn generate(&self) -> Option<MeanDrawdown> {
        self.mean_drawdown.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::time_plus_days;
    use chrono::{DateTime, TimeDelta, Utc};
    use rust_decimal_macros::dec;

    #[test]
    fn test_mean_drawdown_generator_update() {
        struct TestCase {
            input: Drawdown,
            expected_state: MeanDrawdownGenerator,
            expected_output: Option<MeanDrawdown>,
        }

        let base_time = DateTime::<Utc>::MIN_UTC;

        let mut generator = MeanDrawdownGenerator::default();

        let cases = vec![
            // TC0: first ever drawdown
            TestCase {
                input: Drawdown {
                    value: dec!(-0.5), // -50/100
                    time_start: base_time,
                    time_end: time_plus_days(base_time, 2),
                },
                expected_state: MeanDrawdownGenerator {
                    count: 1,
                    mean_drawdown: Some(MeanDrawdown {
                        mean_drawdown: dec!(-0.5),
                        mean_drawdown_ms: TimeDelta::days(2).num_milliseconds(),
                    }),
                },
                expected_output: Some(MeanDrawdown {
                    mean_drawdown: dec!(-0.5),
                    mean_drawdown_ms: TimeDelta::days(2).num_milliseconds(),
                }),
            },
            // TC1: second drawdown updates mean
            TestCase {
                input: Drawdown {
                    value: dec!(-0.5), // -100/200
                    time_start: base_time,
                    time_end: time_plus_days(base_time, 2),
                },
                expected_state: MeanDrawdownGenerator {
                    count: 2,
                    mean_drawdown: Some(MeanDrawdown {
                        mean_drawdown: dec!(-0.5),
                        mean_drawdown_ms: TimeDelta::days(2).num_milliseconds(),
                    }),
                },
                expected_output: Some(MeanDrawdown {
                    mean_drawdown: dec!(-0.5),
                    mean_drawdown_ms: TimeDelta::days(2).num_milliseconds(),
                }),
            },
            // TC2: third drawdown with different duration
            TestCase {
                input: Drawdown {
                    value: dec!(-0.18), // -180/1000
                    time_start: base_time,
                    time_end: time_plus_days(base_time, 5),
                },
                expected_state: MeanDrawdownGenerator {
                    count: 3,
                    mean_drawdown: Some(MeanDrawdown {
                        mean_drawdown: dec!(-0.3933333333333333333333333333), // -59/150
                        mean_drawdown_ms: TimeDelta::days(3).num_milliseconds(),
                    }),
                },
                expected_output: Some(MeanDrawdown {
                    mean_drawdown: dec!(-0.3933333333333333333333333333), // -59/150
                    mean_drawdown_ms: TimeDelta::days(3).num_milliseconds(),
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            generator.update(&test.input);

            // Verify both internal state and generated output
            assert_eq!(
                generator, test.expected_state,
                "TC{index} generator state failed"
            );
            assert_eq!(
                generator.generate(),
                test.expected_output,
                "TC{index} generated output failed"
            );
        }
    }
}
