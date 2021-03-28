use crate::execution::fill::FillEvent;
use crate::execution::error::ExecutionError;
use chrono::Utc;
use crate::portfolio::order::OrderEvent;

/// Generates a result FillEvent by executing an OrderEvent.
pub trait FillGenerator {
    /// Return a FillEvent from executing the input OrderEvent.
    fn generate_fill(&self, order: &OrderEvent) -> Result<FillEvent, ExecutionError>;
}

/// Simulated execution handler that executes OrderEvents to generate FillEvents via a
/// simulated broker interaction.
pub struct SimulatedExecution {
}

impl FillGenerator for SimulatedExecution {
    fn generate_fill(&self, order: &OrderEvent) -> Result<FillEvent, ExecutionError> {
        // Assume for now that all orders are filled at the market price
        Ok(FillEvent::builder()
            .trace_id(order.trace_id)
            .timestamp(Utc::now())
            .symbol(order.symbol.clone())
            .exchange(order.exchange.clone())
            .quantity(order.quantity)
            .decision(order.decision.clone())
            .fill_value_gross(SimulatedExecution::calculate_fill_value_gross(&order))
            .exchange_fee(0.0)
            .slippage_fee(0.0)
            .network_fee(0.0)
            .build()?
        )
    }
}

impl SimulatedExecution {
    /// Constructs a new SimulatedExecution component.
    pub fn new() -> Self {
        SimulatedExecution{}
    }

    /// Calculates the simulated gross fill value (excluding TotalFees) based on the input OrderEvent.
    fn calculate_fill_value_gross(order: &OrderEvent) -> f64 {
        order.quantity.abs() * order.close
    }
}


