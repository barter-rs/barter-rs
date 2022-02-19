use crate::data::error::DataError;
use crate::data::handler::{Continuation, Continuer, MarketGenerator};
use crate::data::MarketEvent;
use barter_data::model::{Candle, MarketData};
use barter_data::ExchangeClient;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedReceiver;
use uuid::Uuid;

/// Configuration for constructing a [`LiveCandleHandler`] via the new() constructor method.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Config {
    pub symbol: String,
    pub interval: String,
}

/// [`MarketEvent`] data handler that consumes a live UnboundedReceiverStream of [`Candle`]s.
/// Implements [`Continuer`] & [`MarketGenerator`].
#[derive(Debug)]
pub struct LiveCandleHandler {
    pub exchange: &'static str,
    pub symbol: String,
    pub interval: String,
    candle_rx: UnboundedReceiver<Candle>,
    can_continue: Continuation,
}

impl Continuer for LiveCandleHandler {
    fn can_continue(&self) -> &Continuation {
        &self.can_continue
    }
}

impl MarketGenerator for LiveCandleHandler {
    fn generate_market(&mut self) -> Option<MarketEvent> {
        // Consume next Candle & generate Some(MarketEvent)
        self.candle_rx
            .blocking_recv()
            .map(|candle| {
                MarketEvent::new(self.exchange, &self.symbol, MarketData::Candle(candle))
            })
            .or_else(|| {
                self.can_continue = Continuation::Stop;
                None
            })
    }
}

impl LiveCandleHandler {
    /// Constructs a new [`LiveCandleHandler`] component using the provided [`Config`]. The injected
    /// [`ExchangeClient`] is used to subscribe to the [`Candle`] stream used by the handler.
    pub async fn init<Client: ExchangeClient>(cfg: Config, mut exchange_client: Client) -> Self {
        // Subscribe to Candle stream via exchange Client
        let candle_rx = exchange_client
            .consume_candles(cfg.symbol.clone(), &cfg.interval)
            .await
            .expect("Failed to consume_candles via exchange Client instance");

        Self {
            exchange: Client::EXCHANGE_NAME,
            symbol: cfg.symbol,
            interval: cfg.interval,
            candle_rx,
            can_continue: Continuation::Continue,
        }
    }

    /// Returns a [`LiveCandleHandlerBuilder`] instance.
    pub fn builder() -> LiveCandleHandlerBuilder {
        LiveCandleHandlerBuilder::new()
    }
}

/// Builder to construct [`LiveCandleHandler`] instances.
#[derive(Debug, Default)]
pub struct LiveCandleHandlerBuilder {
    pub exchange: Option<&'static str>,
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub candle_rx: Option<UnboundedReceiver<Candle>>,
}

impl LiveCandleHandlerBuilder {
    pub fn new() -> Self {
        Self::default()
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

    pub fn interval(self, value: String) -> Self {
        Self {
            interval: Some(value),
            ..self
        }
    }

    pub fn candle_rx(self, value: UnboundedReceiver<Candle>) -> Self {
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