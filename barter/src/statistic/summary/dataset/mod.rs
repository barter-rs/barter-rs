use crate::statistic::{algorithm::welford_online, summary::dataset::dispersion::Dispersion};
use serde::{Deserialize, Serialize};

pub mod dispersion;

#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct DataSetSummary {
    pub count: u64,
    pub sum: f64,
    pub mean: f64,
    pub dispersion: Dispersion,
}

impl DataSetSummary {
    pub fn update(&mut self, next_value: f64) {
        // Increment counter
        self.count += 1;

        // Update Sum
        self.sum += next_value;

        // Update Mean
        let prev_mean = self.mean;
        self.mean = welford_online::calculate_mean(self.mean, next_value, self.count as f64);

        // Update Dispersion
        self.dispersion
            .update(prev_mean, self.mean, next_value, self.count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::statistic::summary::dataset::dispersion::Range;

    #[test]
    fn update_data_summary_with_position() {
        struct TestCase {
            input_next_value: f64,
            expected_summary: DataSetSummary,
        }

        let mut data_summary = DataSetSummary::default();

        let test_cases = vec![
            TestCase {
                // Test case 0
                input_next_value: 1.1,
                expected_summary: DataSetSummary {
                    count: 1,
                    sum: 1.1,
                    mean: 1.1,
                    dispersion: Dispersion {
                        range: Range {
                            activated: true,
                            high: 1.1,
                            low: 1.1,
                        },
                        recurrence_relation_m: 0.00,
                        variance: 0.0,
                        std_dev: 0.0,
                    },
                },
            },
            TestCase {
                // Test case 1
                input_next_value: 1.2,
                expected_summary: DataSetSummary {
                    count: 2,
                    sum: 2.3,
                    mean: (2.3 / 2.0),
                    dispersion: Dispersion {
                        range: Range {
                            activated: true,
                            high: 1.2,
                            low: 1.1,
                        },
                        recurrence_relation_m: 0.005,
                        variance: 0.0025,
                        std_dev: 0.05,
                    },
                },
            },
            TestCase {
                // Test case 2
                input_next_value: 1.3,
                expected_summary: DataSetSummary {
                    count: 3,
                    sum: (2.3 + 1.3),
                    mean: (3.6 / 3.0),
                    dispersion: Dispersion {
                        range: Range {
                            activated: true,
                            high: 1.3,
                            low: 1.1,
                        },
                        recurrence_relation_m: 0.02,
                        variance: 1.0 / 150.0,
                        std_dev: (6.0_f64.sqrt() / 30.0),
                    },
                },
            },
        ];

        for (index, test) in test_cases.into_iter().enumerate() {
            data_summary.update(test.input_next_value);
            assert_eq!(
                data_summary.count, test.expected_summary.count,
                "Count Input: {:?}",
                index
            );
            assert_eq!(
                data_summary.sum, test.expected_summary.sum,
                "Sum Input: {:?}",
                index
            );
            assert_eq!(
                data_summary.mean, test.expected_summary.mean,
                "Mean Input: {:?}",
                index
            );

            let recurrence_diff = data_summary.dispersion.recurrence_relation_m
                - test.expected_summary.dispersion.recurrence_relation_m;
            assert!(recurrence_diff < 1e-10, "Recurrence Input: {:?}", index);

            let variance_diff =
                data_summary.dispersion.variance - test.expected_summary.dispersion.variance;
            assert!(variance_diff < 1e-10, "Variance Input: {:?}", index);

            let std_dev_diff =
                data_summary.dispersion.std_dev - test.expected_summary.dispersion.std_dev;
            assert!(std_dev_diff < 1e-10, "Std. Dev. Input: {:?}", index);
        }
    }
}
