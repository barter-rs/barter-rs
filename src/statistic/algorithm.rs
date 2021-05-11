/// Grouping of Welford Online algorithms for calculating running values from one pass through.
/// See link:
/// https://en.wikipedia.org/wiki/Algorithms_for_calculating_variance#Welford's_online_algorithm
pub struct WelfordOnline;

impl WelfordOnline {
    /// Calculates the next mean.
    pub fn calculate_mean<T>(mut prev_mean: T, next_value: T, count: T) -> T
    where T: Copy + std::ops::Sub<Output = T> + std::ops::Div<Output = T> + std::ops::AddAssign {
        prev_mean += (next_value - prev_mean) / count;
        prev_mean
    }

    /// Calculates the next Welford Online recurrence relation M.
    pub fn calculate_recurrence_relation_m(prev_m: f64, prev_mean: f64, new_value: f64, new_mean: f64) -> f64 {
        prev_m + ((new_value - prev_mean) * (new_value - new_mean))
    }

    /// Calculates the next unbiased 'Sample' Variance using Bessel's correction (count - 1), and the
    /// Welford Online recurrence relation M.
    pub fn calculate_sample_variance(recurrence_relation_m: f64, count: usize) -> f64 {
        match count < 2 {
            true => {
                0.0
            }
            false => {
                recurrence_relation_m / (count as f64 - 1.0)
            }
        }
    }

    /// Calculates the next biased 'Population' Variance using the Welford Online recurrence relation M.
    pub fn calculate_population_variance(recurrence_relation_m: f64, count: usize) -> f64 {
        match count < 1 {
            true => {
                0.0
            }
            false => {
                recurrence_relation_m / count as f64
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_mean() {
        let inputs = vec![
            (0, 100, 1), (100, 1000, 2), (550, 400, 3), (500, 540, 4), (510, 1200, 5), (648, 660, 6) // 650
        ];

        let expected = vec![100, 550, 500, 510, 648, 650];

        for (input, expected) in inputs.iter().zip(expected.into_iter()) {
            let actual_mean = WelfordOnline::calculate_mean(input.0, input.1, input.2);
            assert_eq!(actual_mean, expected);
        }
    }

    #[test]
    fn calculate_recurrence_relation_m() {
        // -- INPUTS --
        // dataset = [10, 100, -10]
        let input_1 = (0.0, 0.0, 10.0, 10.0);
        let input_2 = (0.0, 10.0, 100.0, 55.0);
        let input_3 = (4050.0, 55.0, -10.0, (100.0/3.0));

        // dataset = [-5, -50, -1000]
        let input_4 = (0.0, 0.0, -5.0, -5.0);
        let input_5 = (0.0, -5.0, -50.0, (-55.0/2.0));
        let input_6 = (1012.5, (-55.0/2.0), -1000.0, (-1055.0/3.0));

        // dataset = [90000, -90000, 0]
        let input_7 = (0.0, 0.0, 90000.0, 90000.0);
        let input_8 = (0.0, 90000.0, -90000.0, 0.0);
        let input_9 = (16200000000.0, 0.0, 0.0, 0.0);

        let inputs = vec![
            input_1, input_2, input_3, input_4, input_5, input_6, input_7, input_8, input_9
        ];

        // -- EXPECTED OUTPUTS --
        let expected_1 = 0.0;
        let expected_2 = 4050.0;
        let expected_3 = 20600.0/3.0;
        let expected_4 = 0.0;
        let expected_5 = 1012.5;
        let expected_6 = 1894550.0/3.0;
        let expected_7 = 0.0;
        let expected_8 = 16200000000.0;
        let expected_9 = 16200000000.0;

        let expected = vec![
            expected_1, expected_2, expected_3, expected_4, expected_5, expected_6, expected_7, expected_8, expected_9
        ];

        // -- ASSERT ACTUAL EQUALS EXPECTED --
        for (input, expected) in inputs.iter().zip(expected.into_iter()) {

            let actual_m = WelfordOnline::calculate_recurrence_relation_m(
                input.0, input.1, input.2, input.3);

            assert_eq!(actual_m, expected)
        }
    }

    #[test]
    fn calculate_sample_variance() {
        let inputs = vec![
            (0.0, 1), (1050.0, 5), (1012.5, 123223), (16200000000.0, 3), (99999.9999, 23232)
        ];
        let expected = vec![0.0, 262.5, (675.0/82148.0), 8100000000.0, 4.304592996427187];

        for (input, expected) in inputs.iter().zip(expected.into_iter()) {
            let actual_variance = WelfordOnline::calculate_sample_variance(input.0, input.1);
            assert_eq!(actual_variance, expected);
        }
    }

    #[test]
    fn calculate_population_variance() {
        let inputs = vec![
            (0.0, 1), (1050.0, 5), (1012.5, 123223), (16200000000.0, 3), (99999.9999, 23232)
        ];
        let expected = vec![0.0, 210.0, (1012.5/123223.0), 5400000000.0, 0.4304592996427187];

        for (input, expected) in inputs.iter().zip(expected.into_iter()) {
            let actual_variance = WelfordOnline::calculate_population_variance(input.0, input.1);
            assert_eq!(actual_variance, expected);
        }
    }
}