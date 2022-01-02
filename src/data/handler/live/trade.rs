use crate::data::error::DataError;
use crate::data::handler::{Continuation, Continuer, MarketGenerator};
use crate::data::market::MarketEvent;
use barter_data::model::{MarketData, Trade};
use barter_data::ExchangeClient;
use chrono::Utc;
use serde::Deserialize;
use std::sync::mpsc::{channel, Receiver};
use tokio_stream::StreamExt;
use tracing::debug;
use uuid::Uuid;

/// Configuration for constructing a [LiveTradeHandler] via the new() constructor method.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub symbol: String,
}

/// [MarketEvent] data handler that consumes a live UnboundedReceiverStream of [Trade]s. Implements
/// [Continuer] & [MarketGenerator].
#[derive(Debug)]
pub struct LiveTradeHandler {
    pub exchange: &'static str,
    pub symbol: String,
    trade_rx: Receiver<Trade>,
    can_continue: Continuation,
}

impl Continuer for LiveTradeHandler {
    fn can_continue(&self) -> &Continuation {
        &self.can_continue
    }
}

impl MarketGenerator for LiveTradeHandler {
    fn generate_market(&mut self) -> Option<MarketEvent> {
        // Consume next Trade
        let trade = match self.trade_rx.recv() {
            Ok(trade) => trade,
            Err(_) => {
                self.can_continue = Continuation::Stop;
                return None;
            }
        };

        Some(MarketEvent {
            event_type: MarketEvent::EVENT_TYPE,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            exchange: self.exchange,
            symbol: self.symbol.clone(),
            data: MarketData::Trade(trade),
        })
    }
}

impl LiveTradeHandler {
    /// Constructs a new [LiveTradeHandler] component using the provided [Config]. The injected
    /// [ExchangeClient] is used to subscribe to a [Trade] stream. An asynchronous task is spawned
    /// to consume [Trade]s and route them to the [LiveTradeHandler]'s sync::mpsc::Receiver.
    pub async fn init<Client: ExchangeClient>(cfg: Config, mut exchange_client: Client) -> Self {
        // Subscribe to Trade stream via exchange Client
        let mut trade_stream = exchange_client
            .consume_trades(cfg.symbol.clone())
            .await
            .expect("Failed to consume_trades via exchange Client instance");

        // Spawn Tokio task to async consume_trades from Client and transmit to a sync trade_rx
        let (trade_tx, trade_rx) = channel();
        tokio::spawn(async move {
            loop {
                // Send any received Trades from Client to the LiveTradeHandler trade_rx
                if let Some(trade) = trade_stream.next().await {
                    if trade_tx.send(trade).is_err() {
                        debug!("LiveTradeHandler receiver for Trades has been dropped - closing channel");
                        return;
                    }
                }
            }
        });

        Self {
            exchange: Client::EXCHANGE_NAME,
            symbol: cfg.symbol,
            trade_rx,
            can_continue: Continuation::Continue,
        }
    }

    /// Returns a [LiveTradeHandlerBuilder] instance.
    pub fn builder() -> LiveTradeHandlerBuilder {
        LiveTradeHandlerBuilder::new()
    }
}

/// Builder to construct [LiveTradeHandler] instances.
#[derive(Debug, Default)]
pub struct LiveTradeHandlerBuilder {
    pub exchange: Option<&'static str>,
    pub symbol: Option<String>,
    pub trade_rx: Option<Receiver<Trade>>,
}

impl LiveTradeHandlerBuilder {
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

    pub fn trade_rx(self, value: Receiver<Trade>) -> Self {
        Self {
            trade_rx: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<LiveTradeHandler, DataError> {
        let exchange = self.exchange.ok_or(DataError::BuilderIncomplete)?;
        let symbol = self.symbol.ok_or(DataError::BuilderIncomplete)?;
        let trade_rx = self.trade_rx.ok_or(DataError::BuilderIncomplete)?;

        Ok(LiveTradeHandler {
            exchange,
            symbol,
            trade_rx,
            can_continue: Continuation::Continue,
        })
    }
}