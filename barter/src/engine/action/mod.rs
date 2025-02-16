use crate::engine::{
    action::{
        generate_algo_orders::GenerateAlgoOrdersOutput,
        send_requests::{SendCancelsAndOpensOutput, SendRequestsOutput},
    },
    error::UnrecoverableEngineError,
};
use barter_execution::order::request::{RequestCancel, RequestOpen};
use barter_instrument::{exchange::ExchangeIndex, instrument::InstrumentIndex};
use barter_integration::collection::one_or_many::OneOrMany;
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Defines the `Engine` action for cancelling open order requests.
pub mod cancel_orders;

/// Defines the `Engine` action for generating and sending order requests for closing open positions.
pub mod close_positions;

/// Defines the `Engine` action for generating and sending algorithmic order requests.
pub mod generate_algo_orders;

/// Defines the `Engine` action for sending order `ExecutionRequests` to the execution manager.
pub mod send_requests;

/// Output of the `Engine` after actioning a [`Command`](super::command::Command).
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From)]
#[allow(clippy::large_enum_variant)]
pub enum ActionOutput<ExchangeKey = ExchangeIndex, InstrumentKey = InstrumentIndex> {
    GenerateAlgoOrders(GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>),
    CancelOrders(SendRequestsOutput<RequestCancel, ExchangeKey, InstrumentKey>),
    OpenOrders(SendRequestsOutput<RequestOpen, ExchangeKey, InstrumentKey>),
    ClosePositions(SendCancelsAndOpensOutput<ExchangeKey, InstrumentKey>),
}

impl<ExchangeKey, InstrumentKey> ActionOutput<ExchangeKey, InstrumentKey> {
    /// Returns any unrecoverable errors that occurred during an `Engine` action.
    pub fn unrecoverable_errors(&self) -> Option<OneOrMany<UnrecoverableEngineError>> {
        match self {
            ActionOutput::GenerateAlgoOrders(algo) => algo.cancels_and_opens.unrecoverable_errors(),
            ActionOutput::CancelOrders(cancels) => cancels.unrecoverable_errors(),
            ActionOutput::OpenOrders(opens) => opens.unrecoverable_errors(),
            ActionOutput::ClosePositions(requests) => requests.unrecoverable_errors(),
        }
        .into_option()
    }
}
