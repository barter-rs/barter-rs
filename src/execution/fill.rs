use crate::data::market::MarketMeta;
use crate::execution::error::ExecutionError;
use crate::strategy::signal::Decision;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Fills are journals of work done by an Execution handler. These are sent back to the portfolio
/// so it can apply updates.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct FillEvent {
    pub event_type: &'static str,
    pub trace_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub exchange: &'static str,
    pub symbol: String,
    pub market_meta: MarketMeta, // Metadata propagated from source MarketEvent
    pub decision: Decision,      // LONG, CloseLong, SHORT or CloseShort
    pub quantity: f64,           // +ve or -ve Quantity depending on Decision
    pub fill_value_gross: f64,   // abs(Quantity) * ClosePrice, excluding TotalFees
    pub fees: Fees,
}

impl Default for FillEvent {
    fn default() -> Self {
        Self {
            event_type: FillEvent::EVENT_TYPE,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            exchange: "binance",
            symbol: String::from("eth_usdt"),
            market_meta: Default::default(),
            decision: Decision::default(),
            quantity: 1.0,
            fill_value_gross: 100.0,
            fees: Fees::default(),
        }
    }
}

impl FillEvent {
    pub const EVENT_TYPE: &'static str = "Fill";

    /// Returns a [`FillEventBuilder`] instance.
    pub fn builder() -> FillEventBuilder {
        FillEventBuilder::new()
    }
}

/// All potential fees incurred by a [`FillEvent`].
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Default, Deserialize, Serialize)]
pub struct Fees {
    /// Fee taken by the exchange/broker (eg/ commission).
    pub exchange: FeeAmount,
    /// Order book slippage modelled as a fee.
    pub slippage: FeeAmount,
    /// Fee incurred by any required network transactions (eg/ GAS).
    pub network: FeeAmount,
}

impl Fees {
    /// Calculates the sum of every [FeeAmount] in [Fees].
    pub fn calculate_total_fees(&self) -> f64 {
        self.exchange + self.network + self.slippage
    }
}

/// Communicative type alias for Fee amount as f64.
pub type FeeAmount = f64;

/// Builder to construct [FillEvent] instances.
#[derive(Debug, Default)]
pub struct FillEventBuilder {
    pub trace_id: Option<Uuid>,
    pub timestamp: Option<DateTime<Utc>>,
    pub exchange: Option<&'static str>,
    pub symbol: Option<String>,
    pub market_meta: Option<MarketMeta>,
    pub decision: Option<Decision>,
    pub quantity: Option<f64>,
    pub fill_value_gross: Option<f64>,
    pub fees: Option<Fees>,
}

impl FillEventBuilder {
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

    pub fn symbol(self, value: &'static str) -> Self {
        Self {
            exchange: Some(value),
            ..self
        }
    }

    pub fn market_meta(self, value: MarketMeta) -> Self {
        Self {
            market_meta: Some(value),
            ..self
        }
    }

    pub fn decision(self, value: Decision) -> Self {
        Self {
            decision: Some(value),
            ..self
        }
    }

    pub fn quantity(self, value: f64) -> Self {
        Self {
            quantity: Some(value),
            ..self
        }
    }

    pub fn fill_value_gross(self, value: f64) -> Self {
        Self {
            fill_value_gross: Some(value),
            ..self
        }
    }

    pub fn fees(self, value: Fees) -> Self {
        Self {
            fees: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<FillEvent, ExecutionError> {
        let trace_id = self.trace_id.ok_or(ExecutionError::BuilderIncomplete)?;
        let timestamp = self.timestamp.ok_or(ExecutionError::BuilderIncomplete)?;
        let exchange = self.exchange.ok_or(ExecutionError::BuilderIncomplete)?;
        let symbol = self.symbol.ok_or(ExecutionError::BuilderIncomplete)?;
        let market_meta = self.market_meta.ok_or(ExecutionError::BuilderIncomplete)?;
        let decision = self.decision.ok_or(ExecutionError::BuilderIncomplete)?;
        let quantity = self.quantity.ok_or(ExecutionError::BuilderIncomplete)?;
        let fill_value_gross = self
            .fill_value_gross
            .ok_or(ExecutionError::BuilderIncomplete)?;
        let fees = self.fees.ok_or(ExecutionError::BuilderIncomplete)?;

        Ok(FillEvent {
            event_type: FillEvent::EVENT_TYPE,
            trace_id,
            timestamp,
            exchange,
            symbol,
            market_meta,
            decision,
            quantity,
            fill_value_gross,
            fees,
        })
    }
}
