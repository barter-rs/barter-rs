use super::{exchange::account::ClientAccount, SimulatedEvent};
use crate::ExecutionError;
use tokio::sync::mpsc;

/// [`SimulatedExchange`] account balances, open orders, fees, and latency.
pub mod account;

/// [`SimulatedExchange`] that responds to [`SimulatedEvent`]s.
#[derive(Debug)]
pub struct SimulatedExchange {
    pub event_simulated_rx: mpsc::UnboundedReceiver<SimulatedEvent>,
    pub account: ClientAccount,
}

impl SimulatedExchange {
    /// Construct a [`ExchangeBuilder`] for configuring a new [`SimulatedExchange`].
    pub fn builder() -> ExchangeBuilder {
        ExchangeBuilder::new()
    }

    /// Run the [`SimulatedExchange`] by responding to [`SimulatedEvent`]s.
    pub async fn run(mut self) {
        while let Some(event) = self.event_simulated_rx.recv().await {
            match event {
                SimulatedEvent::FetchOrdersOpen(response_tx) => {
                    self.account.fetch_orders_open(response_tx)
                }
                SimulatedEvent::FetchBalances(response_tx) => {
                    self.account.fetch_balances(response_tx)
                }
                SimulatedEvent::OpenOrders((open_requests, response_tx)) => {
                    self.account.open_orders(open_requests, response_tx)
                }
                SimulatedEvent::CancelOrders((cancel_requests, response_tx)) => {
                    self.account.cancel_orders(cancel_requests, response_tx)
                }
                SimulatedEvent::CancelOrdersAll(response_tx) => {
                    self.account.cancel_orders_all(response_tx)
                }
                SimulatedEvent::MarketTrade((instrument, trade)) => {
                    self.account.match_orders(instrument, trade)
                }
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct ExchangeBuilder {
    event_simulated_rx: Option<mpsc::UnboundedReceiver<SimulatedEvent>>,
    account: Option<ClientAccount>,
}

impl ExchangeBuilder {
    fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn event_simulated_rx(self, value: mpsc::UnboundedReceiver<SimulatedEvent>) -> Self {
        Self {
            event_simulated_rx: Some(value),
            ..self
        }
    }

    pub fn account(self, value: ClientAccount) -> Self {
        Self {
            account: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<SimulatedExchange, ExecutionError> {
        Ok(SimulatedExchange {
            event_simulated_rx: self.event_simulated_rx.ok_or_else(|| {
                ExecutionError::BuilderIncomplete("event_simulated_rx".to_string())
            })?,
            account: self
                .account
                .ok_or_else(|| ExecutionError::BuilderIncomplete("account".to_string()))?,
        })
    }
}
