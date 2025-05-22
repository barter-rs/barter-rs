use crate::error::JackbotError;
use jackbot_data::{error::DataError, streams::consumer::MarketStreamEvent};
use jackbot_instrument::instrument::InstrumentIndex;
use async_trait::async_trait;
use std::{fs::File, io::{BufRead, BufReader}, marker::PhantomData};

/// Generic interface for loading historical market data for backtests.
#[async_trait]
pub trait DataLoader {
    /// The MarketEvent kind loaded by this loader.
    type Kind: for<'de> serde::Deserialize<'de> + Send + Sync + 'static;

    /// Load the market data into memory.
    async fn load(&self) -> Result<Vec<MarketStreamEvent<InstrumentIndex, Self::Kind>>, JackbotError>;
}

/// Loader for JSON lines formatted market data files.
#[derive(Debug, Clone)]
pub struct JsonLinesLoader<Kind> {
    file_path: String,
    _kind: PhantomData<Kind>,
}

impl<Kind> JsonLinesLoader<Kind> {
    /// Create a new [`JsonLinesLoader`] from a file path.
    pub fn new(file_path: impl Into<String>) -> Self {
        Self { file_path: file_path.into(), _kind: PhantomData }
    }
}

#[async_trait]
impl<Kind> DataLoader for JsonLinesLoader<Kind>
where
    Kind: for<'de> serde::Deserialize<'de> + Send + Sync + 'static,
{
    type Kind = Kind;

    async fn load(&self) -> Result<Vec<MarketStreamEvent<InstrumentIndex, Self::Kind>>, JackbotError> {
        let file = File::open(&self.file_path)
            .map_err(|e| JackbotError::MarketData(DataError::Socket(e.to_string())))?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        for line in reader.lines() {
            let line = line.map_err(|e| JackbotError::MarketData(DataError::Socket(e.to_string())))?;
            let event: MarketStreamEvent<InstrumentIndex, Self::Kind> = serde_json::from_str(&line)
                .map_err(|e| JackbotError::MarketData(DataError::Socket(e.to_string())))?;
            events.push(event);
        }
        Ok(events)
    }
}
