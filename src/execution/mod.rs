use crate::execution::error::ExecutionError;
use crate::execution::fill::FillEvent;
use crate::portfolio::order::OrderEvent;

pub mod error;
pub mod fill;
pub mod handler;

/// Generates a result [`FillEvent`] by executing an [`OrderEvent`].
pub trait FillGenerator {
    /// Return a [`FillEvent`] from executing the input [`OrderEvent`].
    fn generate_fill(&self, order: &OrderEvent) -> Result<FillEvent, ExecutionError>;
}
