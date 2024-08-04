use crate::v2::order::ClientOrderId;
use barter_instrument::{
    asset::{name::AssetNameExchange, AssetIndex},
    instrument::{name::InstrumentNameExchange, InstrumentIndex},
};
use barter_integration::error::SocketError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type IndexedExecutionError = ExecutionError<AssetIndex, InstrumentIndex>;
pub type ExchangeExecutionError = ExecutionError<AssetNameExchange, InstrumentNameExchange>;
pub type ExchangeApiError = ApiError<AssetNameExchange, InstrumentNameExchange>;
pub type IndexedApiError = ApiError<AssetIndex, InstrumentIndex>;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum ExecutionError<AssetKey, InstrumentKey> {
    #[error("{0}")]
    Connectivity(#[from] ConnectivityError),

    #[error("{0}")]
    ApiError(#[from] ApiError<AssetKey, InstrumentKey>),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum ConnectivityError {
    #[error("{0}")]
    Socket(String),

    #[error("ExecutionRequest timed out: {0}")]
    Timeout(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum ApiError<AssetKey, InstrumentKey> {
    #[error("rate limit exceeded")]
    RateLimit,
    #[error("instrument {0} invalid: {1}")]
    InstrumentInvalid(InstrumentKey, String),
    #[error("asset {0} balance insufficient: {1}")]
    BalanceInsufficient(AssetKey, String),
    #[error("order rejected with ClientOrderId: {0}")]
    OrderRejected(ClientOrderId),
    #[error("order already cancelled with ClientOrderId: {0}")]
    OrderAlreadyCancelled(ClientOrderId),
    #[error("order already fully filled with ClientOrderId: {0}")]
    OrderAlreadyFullyFilled(ClientOrderId),
}

impl From<SocketError> for ConnectivityError {
    fn from(value: SocketError) -> Self {
        Self::Socket(value.to_string())
    }
}
