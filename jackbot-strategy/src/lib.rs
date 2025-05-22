//! Strategy trait and helpers for Jackbot.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration parameters for a strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    #[serde(default)]
    pub parameters: HashMap<String, f64>,
}

impl StrategyConfig {
    /// Convenience helper to get a parameter by name.
    pub fn get(&self, key: &str) -> Option<f64> {
        self.parameters.get(key).copied()
    }
}

/// Trait implemented by event-driven strategies.
///
/// Strategies receive events of type `E` and can react to lifecycle
/// hooks such as `on_start` and `on_stop`.
pub trait Strategy<E> {
    /// Called once before the strategy begins processing events.
    fn on_start(&mut self, _config: &StrategyConfig) {}

    /// Handle a single event.
    fn on_event(&mut self, event: &E);

    /// Called when the strategy is shutting down.
    fn on_stop(&mut self) {}
}

/// A simple strategy that records every event it receives.
#[derive(Debug, Default)]
pub struct RecordingStrategy<E> {
    pub events: Vec<E>,
}

impl<E: Clone> Strategy<E> for RecordingStrategy<E> {
    fn on_event(&mut self, event: &E) {
        self.events.push(event.clone());
    }
}

/// Strategy counting the number of processed events.
#[derive(Debug, Default)]
pub struct CountingStrategy {
    pub count: usize,
}

impl<E> Strategy<E> for CountingStrategy {
    fn on_event(&mut self, _event: &E) {
        self.count += 1;
    }
}
