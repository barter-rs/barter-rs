use crate::execution::error::ExecutionError;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::strategy::signal::Decision;
use crate::data::market::MarketMeta;

/// Fills are journals of work done by an execution handler. These are sent back to the portfolio
/// so it can apply updates.
#[derive(Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct FillEvent {
    pub event_type: &'static str,
    pub trace_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub exchange: String,
    pub symbol: String,
    pub market_meta: MarketMeta,    // Metadata propagated from source MarketEvent
    pub decision: Decision,         // LONG, CloseLong, SHORT or CloseShort
    pub quantity: f64,              // +ve or -ve Quantity depending on Decision
    pub fill_value_gross: f64,      // abs(Quantity) * ClosePrice, excluding TotalFees
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
            market_meta: Default::default(),
            decision: Decision::default(),
            quantity: 1.0,
            fill_value_gross: 100.0,
            fees: Fees::default(),
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
#[derive(Debug, Default)]
pub struct FillEventBuilder {
    pub trace_id: Option<Uuid>,
    pub timestamp: Option<DateTime<Utc>>,
    pub exchange: Option<String>,
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

    pub fn exchange(self, value: String) -> Self {
        Self {
            exchange: Some(value),
            ..self
        }
    }

    pub fn symbol(self, value: String) -> Self {
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
        if let (
            Some(trace_id),
            Some(timestamp),
            Some(exchange),
            Some(symbol),
            Some(market_meta),
            Some(decision),
            Some(quantity),
            Some(fill_value_gross),
            Some(fees),
        ) = (
            self.trace_id,
            self.timestamp,
            self.exchange,
            self.symbol,
            self.market_meta,
            self.decision,
            self.quantity,
            self.fill_value_gross,
            self.fees,
        ) {
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
        } else {
            Err(ExecutionError::BuilderIncomplete)
        }
    }
}