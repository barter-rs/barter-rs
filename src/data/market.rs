use crate::data::error::DataError;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use ta::{Close, High, Low, Open, Volume};
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
    pub bar: Bar,
}

impl Default for MarketEvent {
    fn default() -> Self {
        Self {
            event_type: MarketEvent::EVENT_TYPE,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            exchange: String::from("BINANCE"),
            symbol: String::from("ETH-USD"),
            bar: Bar::default(),
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
    pub bar: Option<Bar>,
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

    pub fn bar(self, value: Bar) -> Self {
        Self {
            bar: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<MarketEvent, DataError> {
        let trace_id = self.trace_id.ok_or(DataError::BuilderIncomplete)?;
        let timestamp = self.timestamp.ok_or(DataError::BuilderIncomplete)?;
        let exchange = self.exchange.ok_or(DataError::BuilderIncomplete)?;
        let symbol = self.symbol.ok_or(DataError::BuilderIncomplete)?;
        let bar = self.bar.ok_or(DataError::BuilderIncomplete)?;

        Ok(MarketEvent {
            event_type: MarketEvent::EVENT_TYPE,
            trace_id,
            timestamp,
            exchange,
            symbol,
            bar,
        })
    }
}

/// OHLCV data from a timeframe interval with the associated [DateTime] UTC timestamp.
#[derive(Debug, PartialOrd, PartialEq, Deserialize, Serialize)]
pub struct Bar {
    #[serde(rename = "Date")]
    #[serde(deserialize_with = "datetime_utc_parser")]
    pub timestamp: DateTime<Utc>,
    #[serde(rename = "Open")]
    pub open: f64,
    #[serde(rename = "High")]
    pub high: f64,
    #[serde(rename = "Low")]
    pub low: f64,
    #[serde(rename = "Adj Close")]
    pub close: f64,
    pub volume: f64,
}

impl Default for Bar {
    fn default() -> Self {
        Self {
            timestamp: Utc::now(),
            open: 1000.0,
            high: 1100.0,
            low: 900.0,
            close: 1050.0,
            volume: 1000000000.0,
        }
    }
}

impl Open for Bar {
    fn open(&self) -> f64 {
        self.open
    }
}

impl High for Bar {
    fn high(&self) -> f64 {
        self.high
    }
}

impl Low for Bar {
    fn low(&self) -> f64 {
        self.low
    }
}

impl Close for Bar {
    fn close(&self) -> f64 {
        self.close
    }
}

impl Volume for Bar {
    fn volume(&self) -> f64 {
        self.volume
    }
}

impl Bar {
    /// Returns a [BarBuilder] instance.
    pub fn builder() -> BarBuilder {
        BarBuilder::new()
    }
}

/// Builder to construct [Bar] instances.
#[derive(Debug, Default)]
pub struct BarBuilder {
    timestamp: Option<DateTime<Utc>>,
    open: Option<f64>,
    high: Option<f64>,
    low: Option<f64>,
    close: Option<f64>,
    volume: Option<f64>,
}

impl BarBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn timestamp(self, value: DateTime<Utc>) -> Self {
        Self {
            timestamp: Some(value),
            ..self
        }
    }

    pub fn open(self, value: f64) -> Self {
        Self {
            open: Some(value),
            ..self
        }
    }

    pub fn high(self, value: f64) -> Self {
        Self {
            high: Some(value),
            ..self
        }
    }

    pub fn low(self, value: f64) -> Self {
        Self {
            low: Some(value),
            ..self
        }
    }

    pub fn close(self, value: f64) -> Self {
        Self {
            close: Some(value),
            ..self
        }
    }

    pub fn volume(self, value: f64) -> Self {
        Self {
            volume: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<Bar, DataError> {
        let timestamp = self.timestamp.ok_or(DataError::BuilderIncomplete)?;
        let open = self.open.ok_or(DataError::BuilderIncomplete)?;
        let high = self.high.ok_or(DataError::BuilderIncomplete)?;
        let low = self.low.ok_or(DataError::BuilderIncomplete)?;
        let close = self.close.ok_or(DataError::BuilderIncomplete)?;
        let volume = self.volume.ok_or(DataError::BuilderIncomplete)?;

        // Validate
        if low <= open
            && low <= close
            && low <= high
            && high >= open
            && high >= close
            && volume >= 0.0
            && low >= 0.0
        {
            Ok(Bar {
                timestamp,
                open,
                high,
                low,
                close,
                volume,
            })
        } else {
            Err(DataError::BuilderAttributesInvalid)
        }
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
            .bar(Bar::default())
            .build();
        assert!(ok_result.is_ok());

        let err_result = MarketEvent::builder()
            .trace_id(Uuid::new_v4())
            .timestamp(Utc::now())
            .exchange(String::from("GRAND_EXCHANGE"))
            .symbol(String::from("PANTALOONS"))
            // No bar attribute added to builder
            .build();

        assert!(err_result.is_err())
    }

    #[test]
    fn test_bar_builder_validation() {
        fn assert_valid(
            (timestamp, open, high, low, close, volume): (DateTime<Utc>, f64, f64, f64, f64, f64),
        ) {
            let result = Bar::builder()
                .timestamp(timestamp)
                .open(open)
                .high(high)
                .low(low)
                .close(close)
                .volume(volume)
                .build();
            assert!(result.is_ok())
        }

        fn assert_invalid(
            (timestamp, open, high, low, close, volume): (DateTime<Utc>, f64, f64, f64, f64, f64),
        ) {
            let result = Bar::builder()
                .timestamp(timestamp)
                .open(open)
                .high(high)
                .low(low)
                .close(close)
                .volume(volume)
                .build();
            assert!(result.is_err())
        }

        let valid_records = vec![
            // timestamp, open, high, low, close, volume
            (Utc::now(), 20.0, 25.0, 15.0, 21.0, 7500.0),
            (Utc::now(), 10.0, 10.0, 10.0, 10.0, 10.0),
            (Utc::now(), 0.0, 0.0, 0.0, 0.0, 0.0),
        ];
        for record in valid_records {
            assert_valid(record)
        }

        let invalid_records = vec![
            // timestamp, open, high, low, close, volume
            (Utc::now(), -1.0, 25.0, 15.0, 21.0, 7500.0),
            (Utc::now(), 20.0, -1.0, 15.0, 21.0, 7500.0),
            (Utc::now(), 20.0, 25.0, -1.0, 21.0, 7500.0),
            (Utc::now(), 20.0, 25.0, 15.0, -1.0, -7500.0),
            (Utc::now(), 20.0, 25.0, 15.0, 21.0, -1.0),
            (Utc::now(), 14.9, 25.0, 15.0, 21.0, 7500.0),
            (Utc::now(), 25.1, 25.0, 15.0, 21.0, 7500.0),
            (Utc::now(), 20.0, 25.0, 15.0, 14.9, 7500.0),
            (Utc::now(), 20.0, 25.0, 15.0, 25.1, 7500.0),
            (Utc::now(), 20.0, 15.0, 25.0, 21.0, 7500.0),
        ];
        for record in invalid_records {
            assert_invalid(record)
        }
    }
}
