use crate::data::error::DataError;
use crate::data::handler::{Continuation, Continuer, MarketGenerator};
use crate::data::MarketEvent;
use barter_data::model::{MarketData, Trade};
use barter_data::ExchangeClient;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedReceiver;

/// Configuration for constructing a [`LiveTradeHandler`] via the new() constructor method.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Config {
    pub symbol: String,
}

/// [`MarketEvent`] data handler that consumes a live UnboundedReceiverStream of [`Trade`]s.
/// Implements [`Continuer`] & [`MarketGenerator`].
#[derive(Debug)]
pub struct LiveTradeHandler {
    pub exchange: &'static str,
    pub symbol: String,
    trade_rx: UnboundedReceiver<Trade>,
    can_continue: Continuation,
}

impl Continuer for LiveTradeHandler {
    fn can_continue(&self) -> &Continuation {
        &self.can_continue
    }
}

impl MarketGenerator for LiveTradeHandler {
    fn generate_market(&mut self) -> Option<MarketEvent> {
        // Consume next Trade & generate Some(MarketEvent)
        self.trade_rx
            .blocking_recv()
            .map(|trade| {
                MarketEvent::new(self.exchange, &self.symbol, MarketData::Trade(trade))
            })
            .or_else(|| {
                self.can_continue = Continuation::Stop;
                None
            })
    }
}

impl LiveTradeHandler {
    /// Constructs a new [`LiveTradeHandler`] component using the provided [`Config`]. The injected
    /// [`ExchangeClient`] is used to subscribe to the [`Trade`] stream used by the handler.
    pub async fn init<Client: ExchangeClient>(cfg: Config, mut exchange_client: Client) -> Self {
        // Subscribe to Trade stream via exchange Client
        let trade_rx = exchange_client
            .consume_trades(cfg.symbol.clone())
            .await
            .expect("failed to consume_trades via ExchangeClient instance");

        Self {
            exchange: Client::EXCHANGE_NAME,
            symbol: cfg.symbol,
            trade_rx,
            can_continue: Continuation::Continue,
        }
    }

    /// Returns a [`LiveTradeHandlerBuilder`] instance.
    pub fn builder() -> LiveTradeHandlerBuilder {
        LiveTradeHandlerBuilder::new()
    }
}

/// Builder to construct [`LiveTradeHandler`] instances.
#[derive(Debug, Default)]
pub struct LiveTradeHandlerBuilder {
    pub exchange: Option<&'static str>,
    pub symbol: Option<String>,
    pub trade_rx: Option<UnboundedReceiver<Trade>>,
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

    pub fn trade_rx(self, value: UnboundedReceiver<Trade>) -> Self {
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