use crate::{
    client::ExecutionClient,
    order::{
        request::OrderRequestOpen,
        state::Open,
        Order,
    },
    error::UnindexedOrderError,
};
use jackbot_instrument::{
    exchange::ExchangeId,
    instrument::name::InstrumentNameExchange,
};
use async_trait::async_trait;

/// Unified interface for advanced order execution strategies.
///
/// Implementations may schedule or split a single order request into
/// multiple child orders using the provided `Config`.
#[async_trait]
pub trait OrderExecutionStrategy {
    /// Additional configuration required by the strategy.
    type Config: Send + Sync;

    /// Execute the strategy for the given order request and configuration.
    async fn execute(
        &mut self,
        request: OrderRequestOpen<ExchangeId, &InstrumentNameExchange>,
        config: Self::Config,
    ) -> Vec<Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedOrderError>>>;
}
