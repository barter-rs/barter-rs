use std::fmt::Write;

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

pub static METRIC_ENGINE_TRADER_SIGNAL_FORCE_EXIT_COUNT: MetricMetadata = MetricMetadata {
    name: "engine.trader.signal_force_exit",
    description: "Number of signal force exit events that occurred in the trader",
};

pub static METRIC_ENGINE_TRADER_POSITION_UPDATE_COUNT: MetricMetadata = MetricMetadata {
    name: "engine.trader.position_update",
    description: "Number of position updates that occurred in the trader",
};

pub static METRIC_ENGINE_TRADER_NEW_ORDER_COUNT: MetricMetadata = MetricMetadata {
    name: "engine.trader.new_order",
    description: "Number of new orders that occurred in the trader",
};

pub static METRIC_ENGINE_TRADER_FILL_COUNT: MetricMetadata = MetricMetadata {
    name: "engine.trader.fill",
    description: "Number of fills that occurred in the trader",
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