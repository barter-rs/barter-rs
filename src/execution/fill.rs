use crate::execution::error::ExecutionError;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::strategy::signal::Decision;

/// Fills are journals of work done by an execution handler. These are sent back to the portfolio
/// so it can apply updates.
#[derive(Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct FillEvent {
    pub event_type: &'static str,
    pub trace_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub exchange: String,
    pub symbol: String,
    pub decision: Decision,     // LONG, CloseLong, SHORT or CloseShort
    pub quantity: f64,          // +ve or -ve Quantity depending on Decision
    pub fill_value_gross: f64,  // abs(Quantity) * ClosePrice, excluding TotalFees
    pub fees: Fees,
}

impl Default for FillEvent {
    fn default() -> Self {
        Self {
            event_type: FillEvent::EVENT_TYPE,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            exchange: String::from("BINANCE"),
            symbol: String::from("ETH-USD"),
            decision: Decision::default(),
            quantity: 1.0,
            fill_value_gross: 100.0,
            fees: Fees {
                exchange: 0.0,
                slippage: 0.0,
                network: 0.0
            }
        }
    }
}

impl FillEvent {
    pub const EVENT_TYPE: &'static str = "FillEvent";

    /// Returns a [FillEventBuilder] instance.
    pub fn builder() -> FillEventBuilder {
        FillEventBuilder::new()
    }
}

/// All potential fees incurred by a [FillEvent].
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct Fees {
    /// Fee taken by the exchange/broker (eg/ commission).
    pub exchange: FeeAmount,
    /// Order book slippage modelled as a fee.
    pub slippage: FeeAmount,
    /// Fee incurred by any required network transactions (eg/ GAS).
    pub network: FeeAmount,
}

impl Default for Fees {
    fn default() -> Self {
        Self {
            exchange: 0.0,
            slippage: 0.0,
            network: 0.0
        }
    }
}

impl Fees {
    /// Calculates the sum of every [FeeAmount] in [Fees].
    pub fn calculate_total_fees(&self) -> f64 {
        self.exchange + self.network + self.slippage
    }
}

/// Fee amount as f64.
pub type FeeAmount = f64;

/// Builder to construct [FillEvent] instances.
pub struct FillEventBuilder {
    pub trace_id: Option<Uuid>,
    pub timestamp: Option<DateTime<Utc>>,
    pub symbol: Option<String>,
    pub decision: Option<Decision>,
    pub quantity: Option<f64>,
    pub exchange: Option<String>,
    pub fill_value_gross: Option<f64>,
    pub fees: Option<Fees>,
}

impl FillEventBuilder {
    pub fn new() -> Self {
        Self {
            trace_id: None,
            timestamp: None,
            symbol: None,
            decision: None,
            quantity: None,
            exchange: None,
            fill_value_gross: None,
            fees: None,
        }
    }

    pub fn trace_id(mut self, value: Uuid) -> Self {
        self.trace_id = Some(value);
        self
    }

    pub fn timestamp(mut self, value: DateTime<Utc>) -> Self {
        self.timestamp = Some(value);
        self
    }

    pub fn symbol(mut self, value: String) -> Self {
        self.symbol = Some(value);
        self
    }

    pub fn decision(mut self, value: Decision) -> Self {
        self.decision = Some(value);
        self
    }

    pub fn quantity(mut self, value: f64) -> Self {
        self.quantity = Some(value);
        self
    }

    pub fn exchange(mut self, value: String) -> Self {
        self.exchange = Some(value);
        self
    }

    pub fn fill_value_gross(mut self, value: f64) -> Self {
        self.fill_value_gross = Some(value);
        self
    }

    pub fn fees(mut self, value: Fees) -> Self {
        self.fees = Some(value);
        self
    }

    pub fn build(self) -> Result<FillEvent, ExecutionError> {
        if let (
            Some(trace_id),
            Some(timestamp),
            Some(symbol),
            Some(decision),
            Some(quantity),
            Some(exchange),
            Some(fill_value_gross),
            Some(fees),
        ) = (
            self.trace_id,
            self.timestamp,
            self.symbol,
            self.decision,
            self.quantity,
            self.exchange,
            self.fill_value_gross,
            self.fees,
        ) {
            Ok(FillEvent {
                event_type: FillEvent::EVENT_TYPE,
                trace_id,
                timestamp,
                symbol,
                decision,
                quantity,
                exchange,
                fill_value_gross,
                fees,
            })
        } else {
            Err(ExecutionError::BuilderIncomplete)
        }
    }
}