use crate::data::handler::{Continuation, Continuer, MarketGenerator};
use crate::data::error::DataError;
use crate::data::market::MarketEvent;
use barter_data::ExchangeClient;
use barter_data::client::binance::Binance;
use barter_data::model::{Candle, MarketData};
use barter_data::client::ClientName as ExchangeName;
use tracing::debug;
use serde::Deserialize;
use chrono::Utc;
use std::sync::mpsc::{channel, Receiver};
use tokio_stream::StreamExt;
use uuid::Uuid;

/// Configuration for constructing a [LiveCandleHandler] via the new() constructor method.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub exchange: ExchangeName,
    pub symbol: String,
    pub interval: String,
}

/// [MarketEvent] data handler that consumes a live UnboundedReceiverStream of [Candle]s. Implements
/// [Continuer] & [MarketGenerator].
#[derive(Debug)]
pub struct LiveCandleHandler {
    pub exchange: ExchangeName,
    pub symbol: String,
    pub interval: String,
    candle_rx: Receiver<Candle>,
    can_continue: Continuation,
}

impl Continuer for LiveCandleHandler {
    fn can_continue(&self) -> &Continuation {
        &self.can_continue
    }
}

impl MarketGenerator for LiveCandleHandler {
    fn generate_market(&mut self) -> Option<MarketEvent> {
        // Consume next Candle
        let candle = match self.candle_rx.recv() {
            Ok(candle) => candle,
            Err(_) => {
                self.can_continue = Continuation::Stop;
                return None;
            }
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
}

impl LiveCandleHandler {
    /// Initialises an [ExchangeClient] and [Candle] stream, as well as constructs a new
    /// [LiveCandleHandler] component using the provided [Config] struct, as well
    /// as a [Candle] mpsc::Receiver, and a oneshot::[Receiver] for receiving TerminateCommands.
    pub async fn init(cfg: &Config) -> Self {
        // Determine ExchangeClient type & construct
        let mut exchange_client = match cfg.exchange {
            ExchangeName::Binance => Binance::init(),
        }
            .await
            .expect("Failed to construct exchange Client instance");

        // Subscribe to Candle stream via exchange Client
        let mut candle_stream = exchange_client
            .consume_candles(cfg.symbol.clone(), &cfg.interval)
            .await
            .expect("Failed to consume_candles via exchange Client instance");

        // Spawn Tokio task to async consume_candles from Client and transmit to a sync candle_rx
        let (candle_tx, candle_rx) = channel();
        tokio::spawn(async move {
            loop {
                // Send any received Candles from Client to the LiveCandleHandler candle_rx
                if let Some(candle) = candle_stream.next().await {
                    if candle_tx.send(candle).is_err() {
                        debug!("LiveCandleHandler receiver for Candles has been dropped - closing channel");
                        return;
                    }
                }
            }
        });

        Self {
            exchange: cfg.exchange.clone(),
            symbol: cfg.symbol.clone(),
            interval: cfg.interval.clone(),
            candle_rx,
            can_continue: Continuation::Continue,
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
    pub candle_rx: Option<Receiver<Candle>>,
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

    pub fn candle_rx(self, value: Receiver<Candle>) -> Self {
        Self {
            candle_rx: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<LiveCandleHandler, DataError> {
        let exchange = self.exchange.ok_or(DataError::BuilderIncomplete)?;
        let symbol = self.symbol.ok_or(DataError::BuilderIncomplete)?;
        let interval = self.interval.ok_or(DataError::BuilderIncomplete)?;
        let candle_rx = self.candle_rx.ok_or(DataError::BuilderIncomplete)?;

        Ok(LiveCandleHandler {
            exchange,
            symbol,
            interval,
            candle_rx,
            can_continue: Continuation::Continue,
        })
    }
}