use std::fmt::Write;
use metrics_rs::{IntoLabels, Label};

pub struct MetricMetadata {
    name: &'static str,
    description: &'static str,
}

pub static METRIC_ENGINE_EVENTS_TRADES: MetricMetadata = MetricMetadata {
    name: "engine.events.trades",
    description: "Number of trades that occurred in the engine",
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

pub struct ExchangeLabels {
    pub exchange:  String,
}

impl IntoLabels for ExchangeLabels {
    fn into_labels(self) -> Vec<Label> {
        let mut labels = Vec::with_capacity(1);

        labels.push(Label::new("LABEL_EXCHANGE", self.exchange));
        labels
    }
}