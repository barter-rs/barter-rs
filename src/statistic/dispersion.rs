use crate::statistic::algorithm::WelfordOnline;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Dispersion {
    pub range: Range,
    pub recurrence_relation_m: f64,
    pub variance: f64,
    pub standard_deviation: f64
}

impl Default for Dispersion {
    fn default() -> Self {
        Self {
            range: Range::default(),
            recurrence_relation_m: 0.0,
            variance: 0.0,
            standard_deviation: 0.0
        }
    }
}

impl Dispersion {
    pub fn update(&mut self, prev_mean: f64, new_mean: f64, new_value: f64, value_count: usize) {
        // Update Range
        self.range.update(new_value);

        // Update Welford Online recurrence relation M
        self.recurrence_relation_m = WelfordOnline::calculate_recurrence_relation_m(
            self.recurrence_relation_m, prev_mean, new_value, new_mean);

        // Update Population Variance
        self.variance = WelfordOnline::calculate_population_variance(
            self.recurrence_relation_m, value_count);

        // Update Standard Deviation
        self.standard_deviation = self.variance.sqrt();
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Range {
    pub highest: f64,
    pub lowest: f64,
}

impl Default for Range {
    fn default() -> Self {
        Self {
            highest: 0.0,
            lowest: 0.0,
        }
    }
}

impl Range {
    fn update(&mut self, new_value: f64) {
        if new_value > self.highest {
            self.highest = new_value;
        }

        if new_value < self.lowest {
            self.lowest = new_value;
        }
    }

    fn calculate(&self) -> f64 {
        self.highest - self.lowest
    }
}