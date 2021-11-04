use crate::data::error::DataError;
use barter_data::model::{Candle, MarketData};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

/// Market data & related metadata produced by a data::handler implementation for the Strategy
/// & Portfolio to interpret.
#[derive(Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct MarketEvent {
    pub event_type: &'static str,
    pub trace_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub exchange: String,
    pub symbol: String,
    pub data: MarketData,
}

impl Default for MarketEvent {
    fn default() -> Self {
        Self {
            event_type: MarketEvent::EVENT_TYPE,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            exchange: String::from("BINANCE"),
            symbol: String::from("ETH-USD"),
            data: MarketData::Candle(Candle::default()),
        }
    }
}

impl MarketEvent {
    pub const EVENT_TYPE: &'static str = "MarketEvent";

    /// Returns a [MarketEventBuilder] instance.
    pub fn builder() -> MarketEventBuilder {
        MarketEventBuilder::new()
    }
}

/// Builder to construct [MarketEvent] instances.
#[derive(Debug, Default)]
pub struct MarketEventBuilder {
    pub trace_id: Option<Uuid>,
    pub timestamp: Option<DateTime<Utc>>,
    pub exchange: Option<String>,
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

    pub fn exchange(self, value: String) -> Self {
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
        let trace_id = self.trace_id.ok_or(DataError::BuilderIncomplete)?;
        let timestamp = self.timestamp.ok_or(DataError::BuilderIncomplete)?;
        let exchange = self.exchange.ok_or(DataError::BuilderIncomplete)?;
        let symbol = self.symbol.ok_or(DataError::BuilderIncomplete)?;
        let data = self.data.ok_or(DataError::BuilderIncomplete)?;

        Ok(MarketEvent {
            event_type: MarketEvent::EVENT_TYPE,
            trace_id,
            timestamp,
            exchange,
            symbol,
            data,
        })
    }
}

/// Metadata detailing the [Bar] close price & it's associated timestamp. Used to propagate key
/// market information in downstream Events.
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct MarketMeta {
    /// [Bar] close value from the source [MarketEvent].
    pub close: f64,
    /// [Bar] timestamp from the source [MarketEvent].
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

/// Parse supported timestamp strings to a Chrono [Datetime] UTC timestamp.
fn datetime_utc_parser<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let input: &str = Deserialize::deserialize(deserializer)?;

    // If input &str is a DateTime<Utc>
    let datetime_fixed = match DateTime::parse_from_rfc3339(input) {
        Ok(datetime_fixed) => Some(datetime_fixed),
        Err(_) => None,
    };
    if let Some(datetime_fixed) = datetime_fixed {
        return Ok(DateTime::<Utc>::from(datetime_fixed));
    }

    // If input &str is a NaiveDateTime
    let naive_datetime = match NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S") {
        Ok(naive_datetime) => Some(naive_datetime),
        Err(_) => None,
    };
    if let Some(naive_datetime) = naive_datetime {
        return Ok(DateTime::<Utc>::from_utc(naive_datetime, Utc));
    }

    // If input &str is a NaiveDate
    let naive_date = match NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        Ok(naive_date) => Some(naive_date),
        Err(_) => None,
    };
    if let Some(naive_date) = naive_date {
        return Ok(DateTime::<Utc>::from_utc(naive_date.and_hms(0, 0, 0), Utc));
    }

    Err(D::Error::custom(
        "Timestamp format not supported by deserializer",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_incomplete_attributes_validation() {
        let ok_result = MarketEvent::builder()
            .trace_id(Uuid::new_v4())
            .timestamp(Utc::now())
            .exchange(String::from("GRAND_EXCHANGE"))
            .symbol(String::from("PANTALOONS"))
            .data(MarketData::Candle(Candle::default()))
            .build();
        assert!(ok_result.is_ok());

        let err_result = MarketEvent::builder()
            .trace_id(Uuid::new_v4())
            .timestamp(Utc::now())
            .exchange(String::from("GRAND_EXCHANGE"))
            .symbol(String::from("PANTALOONS"))
            // No MarketData attribute added to builder
            .build();

        assert!(err_result.is_err())
    }
}
