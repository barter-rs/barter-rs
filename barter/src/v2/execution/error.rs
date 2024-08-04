use crate::v2::{instrument::asset::AssetId, order::ClientOrderId};
use barter_data::instrument::InstrumentId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Todo: ensure these error types are keyed by ExchangeId to support multiple exchange execution

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum ExecutionError {
    #[error("todo")]
    X,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum ConnectivityError {
    #[error("todo")]
    Disconnected,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum ApiError {
    #[error("rate limit exceeded")]
    RateLimit,
    #[error("instrument {0} invalid: {1}")]
    InstrumentInvalid(InstrumentId, String),
    #[error("asset {0} balance insufficient: {1}")]
    BalanceInsufficient(AssetId, String),
    #[error("order rejected with ClientOrderId: {0}")]
    OrderRejected(ClientOrderId),
    #[error("order already cancelled with ClientOrderId: {0}")]
    OrderAlreadyCancelled(ClientOrderId),
    #[error("order already fully filled with ClientOrderId: {0}")]
    OrderAlreadyFullyFilled(ClientOrderId),
}
