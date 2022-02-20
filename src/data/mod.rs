use crate::data::error::DataError;
use barter_data::model::MarketData;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

/// Barter data module specific errors.
pub mod error;

/// Handlers for historical and live [`MarketEvent`] data feeds.
pub mod handler;

/// Market data & related metadata produced by a data::handler implementation for the Strategy
/// & Portfolio to interpret.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct MarketEvent {
    pub event_type: &'static str,
    pub trace_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub exchange: &'static str,
    pub symbol: String,
    pub data: MarketData,
}

impl MarketEvent {
    pub const EVENT_TYPE: &'static str = "Market";

    /// Constructs a new [`MarketEvent`] using the provided exchange, symbol, and [`MarketData`].
    pub fn new(exchange: &'static str, symbol: &str, data: MarketData) -> Self {
        Self {
            event_type: Self::EVENT_TYPE,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            exchange,
            symbol: symbol.to_owned(),
            data
        }
    }

    /// Returns a [`MarketEventBuilder`] instance.
    pub fn builder() -> MarketEventBuilder {
        MarketEventBuilder::new()
    }
}

/// Builder to construct [`MarketEvent`] instances.
#[derive(Debug, Default)]
pub struct MarketEventBuilder {
    pub trace_id: Option<Uuid>,
    pub timestamp: Option<DateTime<Utc>>,
    pub exchange: Option<&'static str>,
    pub symbol: Option<String>,
    pub data: Option<MarketData>,
}

impl MarketEventBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trace_id(self, value: Uuid) -> Self {
        Self {
            trace_id: Some(value),
            ..self
        }
    }

    pub fn timestamp(self, value: DateTime<Utc>) -> Self {
        Self {
            timestamp: Some(value),
            ..self
        }
    }

    pub fn exchange(self, value: &'static str) -> Self {
        Self {
            exchange: Some(value),
            ..self
        }
    }

    pub fn symbol(self, value: String) -> Self {
        Self {
            symbol: Some(value),
            ..self
        }
    }

    pub fn data(self, value: MarketData) -> Self {
        Self {
            data: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<MarketEvent, DataError> {
        Ok(MarketEvent {
            event_type: MarketEvent::EVENT_TYPE,
            trace_id: self.trace_id.ok_or(DataError::BuilderIncomplete)?,
            timestamp: self.timestamp.ok_or(DataError::BuilderIncomplete)?,
            exchange: self.exchange.ok_or(DataError::BuilderIncomplete)?,
            symbol: self.symbol.ok_or(DataError::BuilderIncomplete)?,
            data: self.data.ok_or(DataError::BuilderIncomplete)?,
        })
    }
}

/// Metadata detailing the [`Candle`] close price & it's associated timestamp. Used to propagate key
/// market information in downstream Events.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct MarketMeta {
    /// [`Candle`] close value from the source [`MarketEvent`].
    pub close: f64,
    /// [`Candle`] timestamp from the source [`MarketEvent`].
    pub timestamp: DateTime<Utc>,
}

impl Default for MarketMeta {
    fn default() -> Self {
        Self {
            close: 100.0,
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use barter_data::test_util;
    use uuid::Uuid;

    #[test]
    fn test_builder_incomplete_attributes_validation() {
        let ok_result = MarketEvent::builder()
            .trace_id(Uuid::new_v4())
            .timestamp(Utc::now())
            .exchange("Grand Exchange")
            .symbol(String::from("PANTALOONS"))
            .data(MarketData::Candle(test_util::candle()))
            .build();
        assert!(ok_result.is_ok());

        let err_result = MarketEvent::builder()
            .trace_id(Uuid::new_v4())
            .timestamp(Utc::now())
            .exchange("Grand Exchange")
            .symbol(String::from("PANTALOONS"))
            // No MarketData attribute added to builder
            .build();

        assert!(err_result.is_err())
    }
}