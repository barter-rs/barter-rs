use crate::{
    engine::{
        action::send_requests::{SendCancelsAndOpensOutput, SendRequests, SendRequestsOutput},
        error::UnrecoverableEngineError,
        execution_tx::ExecutionTxMap,
        state::order::in_flight_recorder::InFlightRequestRecorder,
        Engine,
    },
    risk::{RiskApproved, RiskManager, RiskRefused},
    strategy::algo::AlgoStrategy,
};
use barter_execution::order::{Order, RequestCancel, RequestOpen};
use barter_integration::collection::{none_one_or_many::NoneOneOrMany, one_or_many::OneOrMany};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub trait GenerateAlgoOrders<ExchangeKey, InstrumentKey> {
    fn generate_algo_orders(&mut self) -> GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>;
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey> {
    pub cancels_and_opens: SendCancelsAndOpensOutput<ExchangeKey, InstrumentKey>,
    pub cancels_refused:
        NoneOneOrMany<RiskRefused<Order<ExchangeKey, InstrumentKey, RequestCancel>>>,
    pub opens_refused: NoneOneOrMany<RiskRefused<Order<ExchangeKey, InstrumentKey, RequestOpen>>>,
}

impl<ExchangeKey, InstrumentKey> GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey> {
    pub fn new(
        cancels: SendRequestsOutput<ExchangeKey, InstrumentKey, RequestCancel>,
        opens: SendRequestsOutput<ExchangeKey, InstrumentKey, RequestOpen>,
        cancels_refused: NoneOneOrMany<
            RiskRefused<Order<ExchangeKey, InstrumentKey, RequestCancel>>,
        >,
        opens_refused: NoneOneOrMany<RiskRefused<Order<ExchangeKey, InstrumentKey, RequestOpen>>>,
    ) -> Self {
        Self {
            cancels_and_opens: SendCancelsAndOpensOutput::new(cancels, opens),
            cancels_refused,
            opens_refused,
        }
    }

    pub fn unrecoverable_errors(&self) -> Option<OneOrMany<UnrecoverableEngineError>> {
        self.cancels_and_opens.unrecoverable_errors().into_option()
    }
}

impl<State, ExecutionTxs, Strategy, Risk, ExchangeKey, InstrumentKey>
    GenerateAlgoOrders<ExchangeKey, InstrumentKey> for Engine<State, ExecutionTxs, Strategy, Risk>
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
