use crate::model::{order::OrderKind, ClientOrderId};
use barter_integration::model::instrument::symbol::Symbol;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, PartialEq, Eq, PartialOrd, Debug, Clone, Deserialize, Serialize)]
pub enum ExecutionError {
    #[error("failed to build component due to missing attributes: {0}")]
    BuilderIncomplete(String),

    #[error("SimulatedExchange error: {0}")]
    Simulated(String),

    #[error("Balance for symbol {0} insufficient to open order")]
    InsufficientBalance(Symbol),

    #[error("failed to find Order with ClientOrderId: {0}")]
    OrderNotFound(ClientOrderId),

    #[error("failed to open Order due to unsupported OrderKind: {0}")]
    UnsupportedOrderKind(OrderKind),
}
