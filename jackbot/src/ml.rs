use serde::{Deserialize, Serialize};
use std::{fs::File, path::Path};

pub trait Model {
    fn predict(&self, features: &[f64]) -> f64;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearModel {
    pub weights: Vec<f64>,
    pub bias: f64,
}

impl Model for LinearModel {
    fn predict(&self, features: &[f64]) -> f64 {
        self.weights
            .iter()
            .zip(features.iter())
            .map(|(w, x)| w * x)
            .sum::<f64>()
            + self.bias
    }
}

impl LinearModel {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        Ok(serde_json::from_reader(file)?)
    }
}
