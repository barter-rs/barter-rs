use crate::statistic::metric::drawdown::Drawdown;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

/// [`MaxDrawdown`] is the largest peak-to-trough decline of PnL (Portfolio, Strategy, Instrument),
/// or asset balance.
///
/// Max Drawdown is a measure of downside risk, with larger values indicating downside movements
/// could be volatile.
///
/// See documentation: <https://www.investopedia.com/terms/m/maximum-drawdown-mdd.asp>
#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize, Constructor)]
pub struct MaxDrawdown(pub Drawdown);

/// [`MaxDrawdown`] generator.
#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize, Constructor)]
pub struct MaxDrawdownGenerator {
    pub max: Option<MaxDrawdown>,
}

impl MaxDrawdownGenerator {
    /// Initialise a [`MaxDrawdownGenerator`] from an initial [`Drawdown`].
    pub fn init(drawdown: Drawdown) -> Self {
        Self {
            max: Some(MaxDrawdown(drawdown)),
        }
    }

    /// Updates the internal [`MaxDrawdown`] using the latest next [`Drawdown`]. If the next
    /// drawdown is larger than the current [`MaxDrawdown`], it supersedes it.
    pub fn update(&mut self, next_drawdown: &Drawdown) {
        let max = match self.max.take() {
            Some(current) => {
                if next_drawdown.value.abs() > current.0.value.abs() {
                    MaxDrawdown(next_drawdown.clone())
                } else {
                    current
                }
            }
            None => MaxDrawdown(next_drawdown.clone()),
        };

        self.max = Some(max);
    }

    /// Generate the current [`MeanDrawdown`], if one exists.
    pub fn generate(&self) -> Option<MaxDrawdown> {
        self.max.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::time_plus_days;
    use chrono::{DateTime, Utc};

    #[test]
    fn test_max_drawdown_generator_update() {
        struct TestCase {
            input: Drawdown,
            expected_state: MaxDrawdownGenerator,
            expected_output: Option<MaxDrawdown>,
        }

        let base_time = DateTime::<Utc>::MIN_UTC;

        let mut generator = MaxDrawdownGenerator::default();

        let cases = vec![
            // TC0: first ever drawdown
            TestCase {
                input: Drawdown {
                    value: -25.0 / 110.0,
                    time_start: base_time,
                    time_end: time_plus_days(base_time, 2),
                },
                expected_state: MaxDrawdownGenerator {
                    max: Some(MaxDrawdown::new(Drawdown {
                        value: -25.0 / 110.0,
                        time_start: base_time,
                        time_end: time_plus_days(base_time, 2),
                    })),
                },
                expected_output: Some(MaxDrawdown::new(Drawdown {
                    value: -25.0 / 110.0,
                    time_start: base_time,
                    time_end: time_plus_days(base_time, 2),
                })),
            },
            // TC1: larger drawdown
            TestCase {
                input: Drawdown {
                    value: -110.0 / 200.0,
                    time_start: base_time,
                    time_end: time_plus_days(base_time, 3),
                },
                expected_state: MaxDrawdownGenerator {
                    max: Some(MaxDrawdown::new(Drawdown {
                        value: -110.0 / 200.0,
                        time_start: base_time,
                        time_end: time_plus_days(base_time, 3),
                    })),
                },
                expected_output: Some(MaxDrawdown::new(Drawdown {
                    value: -110.0 / 200.0,
                    time_start: base_time,
                    time_end: time_plus_days(base_time, 3),
                })),
            },
            // TC2: smaller drawdown
            TestCase {
                input: Drawdown {
                    value: -10.0 / 300.0,
                    time_start: base_time,
                    time_end: time_plus_days(base_time, 3),
                },
                expected_state: MaxDrawdownGenerator {
                    max: Some(MaxDrawdown::new(Drawdown {
                        value: -110.0 / 200.0,
                        time_start: base_time,
                        time_end: time_plus_days(base_time, 3),
                    })),
                },
                expected_output: Some(MaxDrawdown::new(Drawdown {
                    value: -110.0 / 200.0,
                    time_start: base_time,
                    time_end: time_plus_days(base_time, 3),
                })),
            },
            // TC3: largest drawdown
            TestCase {
                input: Drawdown {
                    value: -9999.9 / 10000.0,
                    time_start: base_time,
                    time_end: time_plus_days(base_time, 3),
                },
                expected_state: MaxDrawdownGenerator {
                    max: Some(MaxDrawdown::new(Drawdown {
                        value: -9999.9 / 10000.0,
                        time_start: base_time,
                        time_end: time_plus_days(base_time, 3),
                    })),
                },
                expected_output: Some(MaxDrawdown::new(Drawdown {
                    value: -9999.9 / 10000.0,
                    time_start: base_time,
                    time_end: time_plus_days(base_time, 3),
                })),
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
