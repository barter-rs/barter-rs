use async_trait::async_trait;
use crate::error::DataError;

#[cfg(feature = "databento")]
pub mod databento;

#[async_trait]
pub trait Provider {
    async fn init(&mut self) -> Result<(), DataError>;
}