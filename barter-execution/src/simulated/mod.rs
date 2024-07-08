use crate::{Cancelled, ExecutionError, Open, Order, RequestCancel, RequestOpen, SymbolBalance};
use barter_data::subscription::trade::PublicTrade;
use barter_integration::model::instrument::Instrument;
use tokio::sync::oneshot;

/// Simulated Exchange using public trade `Streams` to model available market liquidity. Liquidity
/// is then used to match to open client orders.
pub mod exchange;

/// Simulated [`ExecutionClient`](crate::ExecutionClient) implementation that integrates with the
/// Barter [`SimulatedExchange`](exchange::SimulatedExchange).
pub mod execution;

/// Events used to communicate with the Barter [`SimulatedExchange`](exchange::SimulatedExchange).
///
/// Two main types of [`SimulatedEvent`]:
/// 1. Request sent from the [`SimulatedExecution`](execution::SimulatedExecution)
///    [`ExecutionClient`](crate::ExecutionClient).
/// 2. Market events used to model available liquidity and trigger matches with open client orders.
#[derive(Debug)]
pub enum SimulatedEvent {
    FetchOrdersOpen(oneshot::Sender<Result<Vec<Order<Open>>, ExecutionError>>),
    FetchBalances(oneshot::Sender<Result<Vec<SymbolBalance>, ExecutionError>>),
    OpenOrders(
        (
            Vec<Order<RequestOpen>>,
            oneshot::Sender<Vec<Result<Order<Open>, ExecutionError>>>,
        ),
    ),
    CancelOrders(
        (
            Vec<Order<RequestCancel>>,
            oneshot::Sender<Vec<Result<Order<Cancelled>, ExecutionError>>>,
        ),
    ),
    CancelOrdersAll(oneshot::Sender<Result<Vec<Order<Cancelled>>, ExecutionError>>),
    MarketTrade((Instrument, PublicTrade)),
}
