use crate::engine::{
    action::{
        generate_algo_orders::GenerateAlgoOrdersOutput,
        send_requests::{SendCancelsAndOpensOutput, SendRequestsOutput},
    },
    error::UnrecoverableEngineError,
};
use barter_execution::order::{RequestCancel, RequestOpen};
use barter_instrument::{exchange::ExchangeIndex, instrument::InstrumentIndex};
use barter_integration::collection::one_or_many::OneOrMany;
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub mod cancel_orders;
pub mod close_positions;
pub mod generate_algo_orders;
pub mod send_requests;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, From)]
pub enum ActionOutput<ExchangeKey = ExchangeIndex, InstrumentKey = InstrumentIndex> {
    GenerateAlgoOrders(GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>),
    CancelOrders(SendRequestsOutput<ExchangeKey, InstrumentKey, RequestCancel>),
    OpenOrders(SendRequestsOutput<ExchangeKey, InstrumentKey, RequestOpen>),
    ClosePositions(SendCancelsAndOpensOutput<ExchangeKey, InstrumentKey>),
}

impl<ExchangeKey, InstrumentKey> ActionOutput<ExchangeKey, InstrumentKey> {
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
