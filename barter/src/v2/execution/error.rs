use crate::v2::{error::IndexError, order::ClientOrderId};
use barter_instrument::{
    asset::{name::AssetNameExchange, AssetIndex},
    instrument::{name::InstrumentNameExchange, InstrumentIndex},
};
use barter_integration::error::SocketError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type IndexedClientError = ClientError<AssetIndex, InstrumentIndex>;
pub type UnindexedClientError = ClientError<AssetNameExchange, InstrumentNameExchange>;
pub type IndexedApiError = ApiError<AssetIndex, InstrumentIndex>;
pub type UnindexedApiError = ApiError<AssetNameExchange, InstrumentNameExchange>;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum ExecutionError {
    #[error("{0}")]
    Client(#[from] IndexedClientError),

    #[error("IndexError: {0}")]
    Index(#[from] IndexError),

    #[error("ExecutionManager config invalid: {0}")]
    Config(String),
}

// Todo: probably lives in barter-execution
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum ClientError<AssetKey, InstrumentKey> {
    #[error("Connectivity: {0}")]
    Connectivity(#[from] ConnectivityError),

    #[error("API: {0}")]
    Api(#[from] ApiError<AssetKey, InstrumentKey>),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum ConnectivityError {
    #[error("{0}")]
    Socket(String),

    #[error("ExecutionRequest timed out")]
    Timeout,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum ApiError<AssetKey, InstrumentKey> {
    #[error("rate limit exceeded")]
    RateLimit,
    #[error("asset {0} invalid: {1}")]
    AssetInvalid(AssetKey, String),
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
    #[error("{0}")]
    Custom(String),
}

impl From<SocketError> for ConnectivityError {
    fn from(value: SocketError) -> Self {
        Self::Socket(value.to_string())
    }
}
