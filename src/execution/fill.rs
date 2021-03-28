use crate::execution::error::ExecutionError;
use crate::execution::error::ExecutionError::BuilderIncomplete;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::strategy::signal::Decision;

/// Fills are journals of work done by the execution handler. These are sent back to the portfolio
/// so any relevant positions can be updated.
#[derive(Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct FillEvent {
    pub trace_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub decision: Decision,     // LONG, CloseLong, SHORT or CloseShort
    pub quantity: f64,          // +ve or -ve Quantity depending on Decision
    pub exchange: String,
    pub fill_value_gross: f64,  // abs(Quantity) * ClosePrice, excluding TotalFees
    pub exchange_fee: f64,      // All fees that Exchange imposes on the FillEvent
    pub slippage_fee: f64,      // Financial consequences of FillEvent Slippage modelled as a fee
    pub network_fee: f64,       // All fees incurred from transacting over the network (DEX) eg/ GAS
}

impl Default for FillEvent {
    fn default() -> Self {
        Self {
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            symbol: String::from("ETH-USD"),
            decision: Decision::default(),
            quantity: 10.0,
            exchange: String::from("BINANCE"),
            fill_value_gross: 10000.0,
            exchange_fee: 250.0,
            slippage_fee: 50.0,
            network_fee: 0.0,
        }
    }
}

impl FillEvent {
    /// Returns a FillEventBuilder instance.
    pub fn builder() -> FillEventBuilder {
        FillEventBuilder::new()
    }

    // Todo: Impl the below and also add rustdocs, unit tests...
    // pub fn parse_direction(&self) -> Result<Direction, PortfolioError> {
    //     if self.decision.is_long_or_close_long() && self.quantity.is_sign_positive() {
    //         Ok(Direction::Long)
    //     } else if self.decision.is_short_or_close_short() && self.quantity.is_sign_negative() {
    //         Ok(Direction::Short)
    //     } else {
    //         Err(PortfolioError::ParseDirectionError())
    //     }
    // }
}

/// Builder to construct FillEvent instances.
pub struct FillEventBuilder {
    pub trace_id: Option<Uuid>,
    pub timestamp: Option<DateTime<Utc>>,
    pub symbol: Option<String>,
    pub decision: Option<Decision>,
    pub quantity: Option<f64>,
    pub exchange: Option<String>,
    pub fill_value_gross: Option<f64>,
    pub exchange_fee: Option<f64>,
    pub slippage_fee: Option<f64>,
    pub network_fee: Option<f64>,
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
            exchange_fee: None,
            slippage_fee: None,
            network_fee: None,
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

    pub fn exchange_fee(mut self, value: f64) -> Self {
        self.exchange_fee = Some(value);
        self
    }

    pub fn slippage_fee(mut self, value: f64) -> Self {
        self.slippage_fee = Some(value);
        self
    }

    pub fn network_fee(mut self, value: f64) -> Self {
        self.network_fee = Some(value);
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
            Some(exchange_fee),
            Some(slippage_fee),
            Some(network_fee),
        ) = (
            self.trace_id,
            self.timestamp,
            self.symbol,
            self.decision,
            self.quantity,
            self.exchange,
            self.fill_value_gross,
            self.exchange_fee,
            self.slippage_fee,
            self.network_fee,
        ) {
            Ok(FillEvent {
                trace_id,
                timestamp,
                symbol,
                decision,
                quantity,
                exchange,
                fill_value_gross,
                exchange_fee,
                slippage_fee,
                network_fee,
            })
        } else {
            Err(BuilderIncomplete())
        }
    }
}