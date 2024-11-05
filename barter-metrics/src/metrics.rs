use std::fmt::Write;
use metrics_rs::{IntoLabels, Label};

pub struct MetricMetadata {
    name: &'static str,
    description: &'static str,
}

pub static METRIC_ENGINE_TRADER_EVENT_COUNT: MetricMetadata = MetricMetadata {
    name: "engine.trader.event",
    description: "Number of events that occurred in the trader",
};

pub static METRIC_ENGINE_TRADER_SIGNAL_LATENCY: MetricMetadata = MetricMetadata {
    name: "engine.trader.signal_latency",
    description: "Latency of signals that occurred in the trader",
};

pub static METRIC_ENGINE_TRADER_SIGNAL_COUNT: MetricMetadata = MetricMetadata {
    name: "engine.trader.signal",
    description: "Number of signals that occurred in the trader",
};

impl MetricMetadata {
    pub fn name(&self) -> String {
        self.name_with_prefix("barter.".to_string())
    }

    pub fn name_with_prefix(&self, mut prefix: String) -> String {
        // This operation must succeed. If an error does occur, let's just ignore it.
        let _ = prefix.write_str(self.name);
        prefix
    }

    pub fn description(&self) -> &'static str {
        self.description
    }
}

pub static LABEL_EXCHANGE: &str = "exchange";