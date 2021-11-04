/// Grouping of [Welford Online](https://en.wikipedia.org/wiki/Algorithms_for_calculating_variance#Welford's_online_algorithm)
/// algorithms for calculating running values such as mean and variance in one pass through.
pub mod welford_online {
    /// Calculates the next mean.
    pub fn calculate_mean<T>(mut prev_mean: T, next_value: T, count: T) -> T
    where
        T: Copy + std::ops::Sub<Output = T> + std::ops::Div<Output = T> + std::ops::AddAssign,
    {
        prev_mean += (next_value - prev_mean) / count;
        prev_mean
    }

    /// Calculates the next Welford Online recurrence relation M.
    pub fn calculate_recurrence_relation_m(
        prev_m: f64,
        prev_mean: f64,
        new_value: f64,
        new_mean: f64,
    ) -> f64 {
        prev_m + ((new_value - prev_mean) * (new_value - new_mean))
    }

    /// Calculates the next unbiased 'Sample' Variance using Bessel's correction (count - 1), and the
    /// Welford Online recurrence relation M.
    pub fn calculate_sample_variance(recurrence_relation_m: f64, count: u64) -> f64 {
        match count < 2 {
            true => 0.0,
            false => recurrence_relation_m / (count as f64 - 1.0),
        }
    }

    /// Calculates the next biased 'Population' Variance using the Welford Online recurrence relation M.
    pub fn calculate_population_variance(recurrence_relation_m: f64, count: u64) -> f64 {
        match count < 1 {
            true => 0.0,
            false => recurrence_relation_m / count as f64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_mean() {
        struct Input {
            prev_mean: f64,
            next_value: f64,
            count: f64,
        }

        let inputs = vec![
            Input {
                prev_mean: 0.0,
                next_value: 0.1,
                count: 1.0,
            },
            Input {
                prev_mean: 0.1,
                next_value: -0.2,
                count: 2.0,
            },
            Input {
                prev_mean: -0.05,
                next_value: -0.05,
                count: 3.0,
            },
            Input {
                prev_mean: -0.05,
                next_value: 0.2,
                count: 4.0,
            },
            Input {
                prev_mean: 0.0125,
                next_value: 0.15,
                count: 5.0,
            },
            Input {
                prev_mean: 0.04,
                next_value: -0.17,
                count: 6.0,
            },
        ];

        let expected = vec![0.1, -0.05, -0.05, 0.0125, 0.04, 0.05];

        for (input, expected) in inputs.iter().zip(expected.into_iter()) {
            let actual =
                welford_online::calculate_mean(input.prev_mean, input.next_value, input.count);
            let mean_diff = actual - expected;

            assert!(mean_diff < 1e-10);
        }
    }

    #[test]
    fn calculate_recurrence_relation_m() {
        struct Input {
            prev_m: f64,
            prev_mean: f64,
            new_value: f64,
            new_mean: f64,
        }

        let inputs = vec![
            // dataset_1 = [10, 100, -10]
            Input {
                prev_m: 0.0,
                prev_mean: 0.0,
                new_value: 10.0,
                new_mean: 10.0,
            },
            Input {
                prev_m: 0.0,
                prev_mean: 10.0,
                new_value: 100.0,
                new_mean: 55.0,
            },
            Input {
                prev_m: 4050.0,
                prev_mean: 55.0,
                new_value: -10.0,
                new_mean: (100.0 / 3.0),
            },
            // dataset_2 = [-5, -50, -1000]
            Input {
                prev_m: 0.0,
                prev_mean: 0.0,
                new_value: -5.0,
                new_mean: -5.0,
            },
            Input {
                prev_m: 0.0,
                prev_mean: -5.0,
                new_value: -50.0,
                new_mean: (-55.0 / 2.0),
            },
            Input {
                prev_m: 1012.5,
                prev_mean: (-55.0 / 2.0),
                new_value: -1000.0,
                new_mean: (-1055.0 / 3.0),
            },
            // dataset_3 = [90000, -90000, 0]
            Input {
                prev_m: 0.0,
                prev_mean: 0.0,
                new_value: 90000.0,
                new_mean: 90000.0,
            },
            Input {
                prev_m: 0.0,
                prev_mean: 90000.0,
                new_value: -90000.0,
                new_mean: 0.0,
            },
            Input {
                prev_m: 16200000000.0,
                prev_mean: 0.0,
                new_value: 0.0,
                new_mean: 0.0,
            },
        ];

        let expected = vec![
            0.0,
            4050.0,
            20600.0 / 3.0,
            0.0,
            1012.5,
            1894550.0 / 3.0,
            0.0,
            16200000000.0,
            16200000000.0,
        ];

        for (input, expected) in inputs.iter().zip(expected.into_iter()) {
            let actual_m = welford_online::calculate_recurrence_relation_m(
                input.prev_m,
                input.prev_mean,
                input.new_value,
                input.new_mean,
            );

            assert_eq!(actual_m, expected)
        }
    }

    #[test]
    fn calculate_sample_variance() {
        // fn calculate_sample_variance(recurrence_relation_m: f64, count: u64) -> f64
        let inputs = vec![
            (0.0, 1),
            (1050.0, 5),
            (1012.5, 123223),
            (16200000000.0, 3),
            (99999.9999, 23232),
        ];
        let expected = vec![
            0.0,
            262.5,
            (675.0 / 82148.0),
            8100000000.0,
            4.304592996427187,
        ];

        for (input, expected) in inputs.iter().zip(expected.into_iter()) {
            let actual_variance = welford_online::calculate_sample_variance(input.0, input.1);
            assert_eq!(actual_variance, expected);
        }
    }

    #[test]
    fn calculate_population_variance() {
        // fn calculate_population_variance(recurrence_relation_m: f64, count: u64) -> f64
        let inputs = vec![
            (0.0, 1),
            (1050.0, 5),
            (1012.5, 123223),
            (16200000000.0, 3),
            (99999.9999, 23232),
        ];
        let expected = vec![
            0.0,
            210.0,
            (1012.5 / 123223.0),
            5400000000.0,
            4.304407709194215,
        ];

        for (input, expected) in inputs.iter().zip(expected.into_iter()) {
            let actual_variance = welford_online::calculate_population_variance(input.0, input.1);
            assert_eq!(actual_variance, expected);
        }
    }
}
