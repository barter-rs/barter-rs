/// Grouping of [Welford Online](https://en.wikipedia.org/wiki/Algorithms_for_calculating_variance#Welford's_online_algorithm)
/// algorithms for calculating running values such as mean and variance in one pass through.
pub mod welford_online {
    use rust_decimal::Decimal;

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
        prev_m: Decimal,
        prev_mean: Decimal,
        new_value: Decimal,
        new_mean: Decimal,
    ) -> Decimal {
        prev_m + ((new_value - prev_mean) * (new_value - new_mean))
    }

    /// Calculates the next unbiased 'Sample' Variance using Bessel's correction (count - 1), and the
    /// Welford Online recurrence relation M.
    pub fn calculate_sample_variance(recurrence_relation_m: Decimal, count: Decimal) -> Decimal {
        match count < Decimal::TWO {
            true => Decimal::ZERO,
            false => recurrence_relation_m / (count - Decimal::ONE),
        }
    }

    /// Calculates the next biased 'Population' Variance using the Welford Online recurrence relation M.
    pub fn calculate_population_variance(
        recurrence_relation_m: Decimal,
        count: Decimal,
    ) -> Decimal {
        match count < Decimal::ONE {
            true => Decimal::ZERO,
            false => recurrence_relation_m / count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use std::str::FromStr;

    #[test]
    fn calculate_mean() {
        struct Input {
            prev_mean: Decimal,
            next_value: Decimal,
            count: Decimal,
            expected: Decimal,
        }

        // dataset = [0.1, -0.2, -0.05, 0.2, 0.15, -0.17]
        let inputs = vec![
            // TC0
            Input {
                prev_mean: dec!(0.0),
                next_value: dec!(0.1),
                count: dec!(1.0),
                expected: dec!(0.1),
            },
            // TC1
            Input {
                prev_mean: dec!(0.1),
                next_value: dec!(-0.2),
                count: dec!(2.0),
                expected: dec!(-0.05),
            },
            // TC2
            Input {
                prev_mean: dec!(-0.05),
                next_value: dec!(-0.05),
                count: dec!(3.0),
                expected: dec!(-0.05),
            },
            // TC3
            Input {
                prev_mean: dec!(-0.05),
                next_value: dec!(0.2),
                count: dec!(4.0),
                expected: dec!(0.0125),
            },
            // TC4
            Input {
                prev_mean: dec!(0.0125),
                next_value: dec!(0.15),
                count: dec!(5.0),
                expected: dec!(0.04),
            },
            // TC5
            Input {
                prev_mean: dec!(0.04),
                next_value: dec!(-0.17),
                count: dec!(6.0),
                expected: dec!(0.005),
            },
        ];

        for (index, test) in inputs.iter().enumerate() {
            let actual =
                welford_online::calculate_mean(test.prev_mean, test.next_value, test.count);

            assert_eq!(actual, test.expected, "TC{index} failed")
        }
    }

    #[test]
    fn calculate_recurrence_relation_m() {
        struct Input {
            prev_m: Decimal,
            prev_mean: Decimal,
            new_value: Decimal,
            new_mean: Decimal,
        }

        let inputs = vec![
            // dataset_1 = [10, 100, -10]
            Input {
                prev_m: dec!(0.0),
                prev_mean: dec!(0.0),
                new_value: dec!(10.0),
                new_mean: dec!(10.0),
            },
            Input {
                prev_m: dec!(0.0),
                prev_mean: dec!(10.0),
                new_value: dec!(100.0),
                new_mean: dec!(55.0),
            },
            Input {
                prev_m: dec!(4050.0),
                prev_mean: dec!(55.0),
                new_value: dec!(-10.0),
                new_mean: Decimal::from_str("33.333333333333333333").unwrap(),
            },
            // dataset_2 = [-5, -50, -1000]
            Input {
                prev_m: dec!(0.0),
                prev_mean: dec!(0.0),
                new_value: dec!(-5.0),
                new_mean: dec!(-5.0),
            },
            Input {
                prev_m: dec!(0.0),
                prev_mean: dec!(-5.0),
                new_value: dec!(-50.0),
                new_mean: dec!(-27.5),
            },
            Input {
                prev_m: dec!(1012.5),
                prev_mean: dec!(-27.5),
                new_value: dec!(-1000.0),
                new_mean: dec!(-351.666666666666666667),
            },
            // dataset_3 = [90000, -90000, 0]
            Input {
                prev_m: dec!(0.0),
                prev_mean: dec!(0.0),
                new_value: dec!(90000.0),
                new_mean: dec!(90000.0),
            },
            Input {
                prev_m: dec!(0.0),
                prev_mean: dec!(90000.0),
                new_value: dec!(-90000.0),
                new_mean: dec!(0.0),
            },
            Input {
                prev_m: dec!(16200000000.0),
                prev_mean: dec!(0.0),
                new_value: dec!(0.0),
                new_mean: dec!(0.0),
            },
        ];

        let expected = vec![
            dec!(0.0),
            dec!(4050.0),
            dec!(6866.6666666666666666450),
            dec!(0.0),
            dec!(1012.5),
            dec!(631516.6666666666666663425),
            dec!(0.0),
            dec!(16200000000.0),
            dec!(16200000000.0),
        ];

        for (index, (input, expected)) in inputs.iter().zip(expected.into_iter()).enumerate() {
            let actual_m = welford_online::calculate_recurrence_relation_m(
                input.prev_m,
                input.prev_mean,
                input.new_value,
                input.new_mean,
            );

            assert_eq!(actual_m, expected, "TC{index} failed");
        }
    }

    #[test]
    fn calculate_sample_variance() {
        let inputs = vec![
            (dec!(0.0), dec!(1)),
            (dec!(1050.0), dec!(5)),
            (dec!(1012.5), dec!(123223)),
            (dec!(16200000000.0), dec!(3)),
            (dec!(99999.9999), dec!(23232)),
        ];
        let expected = vec![
            dec!(0.0),
            dec!(262.5),
            dec!(0.0082168768564055120027267858),
            dec!(8100000000.0),
            dec!(4.3045929964271878093926219276),
        ];

        for ((input_m, input_count), expected) in inputs.iter().zip(expected.into_iter()) {
            let actual_variance = welford_online::calculate_sample_variance(*input_m, *input_count);
            assert_eq!(actual_variance, expected);
        }
    }

    #[test]
    fn calculate_population_variance() {
        let inputs = vec![
            (dec!(0.0), 1),
            (dec!(1050.0), 5),
            (dec!(1012.5), 123223),
            (dec!(16200000000.0), 3),
            (dec!(99999.9999), 23232),
        ];
        let expected = vec![
            dec!(0.0),
            dec!(210.0),
            dec!(0.0082168101734254157097295148),
            dec!(5400000000.0),
            dec!(4.3044077091942148760330578512),
        ];

        for (index, (input, expected)) in inputs.iter().zip(expected.into_iter()).enumerate() {
            let actual_variance =
                welford_online::calculate_population_variance(input.0, input.1.into());
            assert_eq!(actual_variance, expected, "TC{index} failed");
        }
    }
}
