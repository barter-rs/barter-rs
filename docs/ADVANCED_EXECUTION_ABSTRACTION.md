# Advanced Execution Abstraction

Advanced order types such as TWAP, VWAP and "always maker" share common behaviour but previously lacked a unified interface. This document outlines a simple trait for implementing these strategies in a consistent way.

## Goals
- Provide a single trait to drive complex order execution algorithms.
- Keep implementations exchange agnostic by building on `ExecutionClient`.
- Encourage composability and testing of custom strategies.

## `OrderExecutionStrategy`

```rust
use async_trait::async_trait;
use jackbot_execution::order::{Order, request::OrderRequestOpen, state::Open};
use jackbot_execution::client::ExecutionClient;
use jackbot_execution::error::UnindexedOrderError;
use jackbot_instrument::{exchange::ExchangeId, instrument::name::InstrumentNameExchange};

#[async_trait]
pub trait OrderExecutionStrategy {
    /// Additional configuration required by the strategy.
    type Config: Send + Sync;

    /// Execute the strategy for the given order request.
    async fn execute(
        &mut self,
        request: OrderRequestOpen<ExchangeId, &InstrumentNameExchange>,
        config: Self::Config,
    ) -> Vec<Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedOrderError>>>;
}
```

Implementations of this trait may split the request into multiple child orders or schedule placement over time. Parameters specific to each algorithm are supplied via the `Config` associated type.

## Initial Implementations
- `TwapScheduler` and `VwapScheduler` now implement `OrderExecutionStrategy`.
- An `AlwaysMaker` strategy will build upon the same trait in a future update.

This abstraction provides the foundation for more advanced execution logic while keeping existing schedulers intact.
