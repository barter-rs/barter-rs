use crate::data::error::DataError;
use crate::data::handler::{Continuation, Continuer};
use crate::data::market::MarketEvent;
use barter_data::client::binance::Binance;
use barter_data::client::{ClientConfig, ClientName as ExchangeName};
use barter_data::model::{Candle, MarketData};
use barter_data::ExchangeClient;
use chrono::Utc;
use serde::Deserialize;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;
use uuid::Uuid;

/// Configuration for constructing a [LiveCandleHandler] via the new() constructor method.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub rate_limit_per_minute: u64,
    pub exchange: ExchangeName,
    pub symbol: String,
    pub interval: String,
}

/// [MarketEvent] data handler that consumes a live [UnboundedReceiverStream] of [Candle]s. Implements
/// [Continuer] & [MarketGenerator].
pub struct LiveCandleHandler {
    pub exchange: ExchangeName,
    pub symbol: String,
    pub interval: String,
    pub data_stream: UnboundedReceiverStream<Candle>,
}

impl Continuer for LiveCandleHandler {
    fn can_continue(&mut self) -> Continuation {
        Continuation::Continue
    }
}

impl LiveCandleHandler {
    pub async fn generate_market(&mut self) -> Option<MarketEvent> {
        // Consume next candle if it's available
        let candle = match self.data_stream.next().await {
            Some(candle) => candle,
            _ => return None,
        };

        Some(MarketEvent {
            event_type: MarketEvent::EVENT_TYPE,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            exchange: format!("{:?}", self.exchange.clone()),
            symbol: self.symbol.clone(),
            data: MarketData::Candle(candle),
        })
    }

    /// Constructs a new [LiveCandleHandler] component using the provided [Config] struct, as well
    /// as a oneshot::[Receiver] for receiving [TerminateCommand]s.
    pub async fn new(cfg: &Config) -> Self {
        // Determine ExchangeClient instance & construct
        let mut exchange_client = match cfg.exchange {
            ExchangeName::Binance => Binance::new(ClientConfig {
                rate_limit_per_minute: cfg.rate_limit_per_minute,
            }),
        }
        .await
        .unwrap();

        let data_stream = exchange_client
            .consume_candles(cfg.symbol.clone(), &*cfg.interval.clone())
            .await
            .unwrap();

        Self {
            exchange: cfg.exchange.clone(),
            symbol: cfg.symbol.clone(),
            interval: cfg.interval.clone(),
            data_stream,
        }
    }

    /// Returns a [LiveCandleHandlerBuilder] instance.
    pub fn builder() -> LiveCandleHandlerBuilder {
        LiveCandleHandlerBuilder::new()
    }
}

/// Builder to construct [LiveCandleHandler] instances.
#[derive(Debug, Default)]
pub struct LiveCandleHandlerBuilder {
    pub exchange: Option<ExchangeName>,
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub data_stream: Option<UnboundedReceiverStream<Candle>>,
}

impl LiveCandleHandlerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn exchange(self, value: ExchangeName) -> Self {
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

    pub fn interval(self, value: String) -> Self {
        Self {
            interval: Some(value),
            ..self
        }
    }

    pub fn data_stream(self, value: UnboundedReceiverStream<Candle>) -> Self {
        Self {
            data_stream: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<LiveCandleHandler, DataError> {
        let exchange = self.exchange.ok_or(DataError::BuilderIncomplete)?;
        let symbol = self.symbol.ok_or(DataError::BuilderIncomplete)?;
        let interval = self.interval.ok_or(DataError::BuilderIncomplete)?;
        let data_stream = self.data_stream.ok_or(DataError::BuilderIncomplete)?;

        Ok(LiveCandleHandler {
            exchange,
            symbol,
            interval,
            data_stream,
        })
    }
}
