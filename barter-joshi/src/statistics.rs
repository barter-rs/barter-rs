//! Statistics gathering and convergence table for Monte Carlo.
//!
//! Implements the StatisticsGatherer and ConvergenceTable from Joshi Chapter 7.5.

use serde::{Deserialize, Serialize};

/// Online statistics gatherer using Welford's algorithm.
///
/// Tracks mean, variance, min, and max of a stream of values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsGatherer {
    count: u64,
    mean: f64,
    m2: f64,  // sum of squared deviations from mean
    min: f64,
    max: f64,
}

impl StatisticsGatherer {
    pub fn new() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            m2: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }

    /// Add a new observation.
    pub fn add(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn mean(&self) -> f64 {
        self.mean
    }

    /// Sample variance (Bessel-corrected).
    pub fn variance(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        self.m2 / (self.count - 1) as f64
    }

    /// Sample standard deviation.
    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }

    /// Standard error of the mean: σ / √n.
    pub fn std_error(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        self.std_dev() / (self.count as f64).sqrt()
    }

    /// 95% confidence interval half-width: 1.96 * std_error.
    pub fn confidence_95(&self) -> f64 {
        1.96 * self.std_error()
    }

    pub fn min(&self) -> f64 {
        self.min
    }

    pub fn max(&self) -> f64 {
        self.max
    }
}

impl Default for StatisticsGatherer {
    fn default() -> Self {
        Self::new()
    }
}

/// Convergence table: records statistics at powers of 2.
///
/// From Joshi Chapter 7.5. Records the mean estimate after 2, 4, 8, 16, ...
/// simulations, allowing you to observe convergence visually.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceTable {
    pub entries: Vec<ConvergenceEntry>,
    gatherer: StatisticsGatherer,
    next_record: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceEntry {
    pub paths: u64,
    pub mean: f64,
    pub std_error: f64,
}

impl ConvergenceTable {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            gatherer: StatisticsGatherer::new(),
            next_record: 2,
        }
    }

    /// Add a new simulation result.
    pub fn add(&mut self, value: f64) {
        self.gatherer.add(value);
        if self.gatherer.count() == self.next_record {
            self.entries.push(ConvergenceEntry {
                paths: self.next_record,
                mean: self.gatherer.mean(),
                std_error: self.gatherer.std_error(),
            });
            self.next_record *= 2;
        }
    }

    /// Record the final statistics regardless of path count.
    pub fn finalize(&mut self) {
        if self.gatherer.count() > 0
            && self.entries.last().map_or(true, |e| e.paths != self.gatherer.count())
        {
            self.entries.push(ConvergenceEntry {
                paths: self.gatherer.count(),
                mean: self.gatherer.mean(),
                std_error: self.gatherer.std_error(),
            });
        }
    }

    pub fn gatherer(&self) -> &StatisticsGatherer {
        &self.gatherer
    }

    /// Print the convergence table.
    pub fn print(&self) {
        println!("{:>12} {:>14} {:>14}", "Paths", "Mean", "Std Error");
        println!("{:-<42}", "");
        for entry in &self.entries {
            println!(
                "{:>12} {:>14.6} {:>14.6}",
                entry.paths, entry.mean, entry.std_error
            );
        }
    }
}

impl Default for ConvergenceTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_statistics_gatherer() {
        let mut sg = StatisticsGatherer::new();
        for v in [1.0, 2.0, 3.0, 4.0, 5.0] {
            sg.add(v);
        }
        assert_eq!(sg.count(), 5);
        assert!((sg.mean() - 3.0).abs() < 1e-10);
        assert!((sg.variance() - 2.5).abs() < 1e-10);
        assert_eq!(sg.min(), 1.0);
        assert_eq!(sg.max(), 5.0);
    }

    #[test]
    fn test_convergence_table_records_at_powers_of_2() {
        let mut ct = ConvergenceTable::new();
        for i in 0..100 {
            ct.add(i as f64);
        }
        ct.finalize();
        let recorded_paths: Vec<u64> = ct.entries.iter().map(|e| e.paths).collect();
        assert!(recorded_paths.contains(&2));
        assert!(recorded_paths.contains(&4));
        assert!(recorded_paths.contains(&8));
        assert!(recorded_paths.contains(&64));
    }
}
