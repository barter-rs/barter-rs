use crate::{
    engine::{
        Engine,
        action::send_requests::{SendCancelsAndOpensOutput, SendRequests, SendRequestsOutput},
        error::UnrecoverableEngineError,
        execution_tx::ExecutionTxMap,
        state::order::in_flight_recorder::InFlightRequestRecorder,
    },
    risk::{RiskApproved, RiskManager, RiskRefused},
    strategy::algo::AlgoStrategy,
};
use barter_execution::order::request::{
    OrderRequestCancel, OrderRequestOpen, RequestCancel, RequestOpen,
};
use barter_instrument::{exchange::ExchangeIndex, instrument::InstrumentIndex};
use barter_integration::collection::{none_one_or_many::NoneOneOrMany, one_or_many::OneOrMany};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Trait that defines how the [`Engine`] generates and sends algorithmic order requests.
///
/// # Type Parameters
/// * `ExchangeKey` - Type used to identify an exchange (defaults to [`ExchangeIndex`]).
/// * `InstrumentKey` - Type used to identify an instrument (defaults to [`InstrumentIndex`]).
pub trait GenerateAlgoOrders<ExchangeKey = ExchangeIndex, InstrumentKey = InstrumentIndex> {
    /// Generates and sends algorithmic order requests.
    ///
    /// Returns a [`GenerateAlgoOrdersOutput`] containing work done:
    /// - Generated orders that were approved by the [`RiskManager`] and sent for execution.
    /// - Generated cancel requests that were refused by the [`RiskManager`].
    /// - Generated open requests that were refused by the [`RiskManager`].
    fn generate_algo_orders(&mut self) -> GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>;
}

impl<Clock, State, ExecutionTxs, Strategy, Risk, ExchangeKey, InstrumentKey>
    GenerateAlgoOrders<ExchangeKey, InstrumentKey>
    for Engine<Clock, State, ExecutionTxs, Strategy, Risk>
where
    State: InFlightRequestRecorder<ExchangeKey, InstrumentKey>,
    ExecutionTxs: ExecutionTxMap<ExchangeKey, InstrumentKey>,
    Strategy: AlgoStrategy<ExchangeKey, InstrumentKey, State = State>,
    Risk: RiskManager<ExchangeKey, InstrumentKey, State = State>,
    ExchangeKey: Debug + Clone,
    InstrumentKey: Debug + Clone,
{
    fn generate_algo_orders(&mut self) -> GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey> {
        // Generate orders
        let (cancels, opens) = self.strategy.generate_algo_orders(&self.state);

        // RiskApprove & RiskRefuse order requests
        let (cancels, opens, refused_cancels, refused_opens) =
            self.risk.check(&self.state, cancels, opens);

        // Send risk approved order requests
        let cancels = self.send_requests(cancels.into_iter().map(|RiskApproved(cancel)| cancel));
        let opens = self.send_requests(opens.into_iter().map(|RiskApproved(open)| open));

        // Collect remaining Iterators (so we can access &mut self)
        let cancels_refused = refused_cancels.into_iter().collect();
        let opens_refused = refused_opens.into_iter().collect();

        // Record in flight order requests
        self.state.record_in_flight_cancels(cancels.sent.iter());
        self.state.record_in_flight_opens(opens.sent.iter());

        GenerateAlgoOrdersOutput::new(cancels, opens, cancels_refused, opens_refused)
    }
}

/// Summary of work done by the [`Engine`] action [`GenerateAlgoOrders::generate_algo_orders`].
///
/// Contains the complete result of an algorithmic order generation action,
/// including successful and risk-refused orders, as well as any errors that occurred.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct GenerateAlgoOrdersOutput<ExchangeKey = ExchangeIndex, InstrumentKey = InstrumentIndex> {
    /// Generates orders that were approved by the [`RiskManager`] and sent for execution.
    pub cancels_and_opens: SendCancelsAndOpensOutput<ExchangeKey, InstrumentKey>,
    /// Generated cancel requests that were refused by the [`RiskManager`].
    pub cancels_refused: NoneOneOrMany<RiskRefused<OrderRequestCancel<ExchangeKey, InstrumentKey>>>,
    /// Generated open requests that were refused by the [`RiskManager`].
    pub opens_refused: NoneOneOrMany<RiskRefused<OrderRequestOpen<ExchangeKey, InstrumentKey>>>,
}

impl<ExchangeKey, InstrumentKey> GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey> {
    /// Construct a new [`GenerateAlgoOrdersOutput`].
    pub fn new(
        cancels: SendRequestsOutput<RequestCancel, ExchangeKey, InstrumentKey>,
        opens: SendRequestsOutput<RequestOpen, ExchangeKey, InstrumentKey>,
        cancels_refused: NoneOneOrMany<RiskRefused<OrderRequestCancel<ExchangeKey, InstrumentKey>>>,
        opens_refused: NoneOneOrMany<RiskRefused<OrderRequestOpen<ExchangeKey, InstrumentKey>>>,
    ) -> Self {
        Self {
            cancels_and_opens: SendCancelsAndOpensOutput::new(cancels, opens),
            cancels_refused,
            opens_refused,
        }
    }

    /// Returns `true` if no `GenerateAlgoOrdersOutput` is completely empty.
    pub fn is_empty(&self) -> bool {
        self.cancels_and_opens.is_empty()
            && self.cancels_refused.is_none()
            && self.opens_refused.is_none()
    }

    /// Returns any unrecoverable errors that occurred during order request generation & sending.
    pub fn unrecoverable_errors(&self) -> Option<OneOrMany<UnrecoverableEngineError>> {
        self.cancels_and_opens.unrecoverable_errors().into_option()
    }
}

impl<ExchangeKey, InstrumentKey> Default for GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey> {
    fn default() -> Self {
        Self {
            cancels_and_opens: SendCancelsAndOpensOutput::default(),
            cancels_refused: NoneOneOrMany::None,
            opens_refused: NoneOneOrMany::None,
        }
    }
}
