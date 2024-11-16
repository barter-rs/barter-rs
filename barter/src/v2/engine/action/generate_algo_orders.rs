use crate::v2::{
    engine::{
        action::send_requests::SendRequestsOutput,
        error::UnrecoverableEngineError,
        execution_tx::ExecutionTxMap,
        state::{order::in_flight_recorder::InFlightRequestRecorder, EngineState},
        Engine,
    },
    order::{Order, RequestCancel, RequestOpen},
    risk::{RiskApproved, RiskManager, RiskRefused},
    strategy::algo::AlgoStrategy,
};
use barter_integration::collection::{none_one_or_many::NoneOneOrMany, one_or_many::OneOrMany};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub trait GenerateAlgoOrders<ExchangeKey, InstrumentKey> {
    fn generate_algo_orders(&mut self) -> GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>;
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey> {
    pub cancels: SendRequestsOutput<ExchangeKey, InstrumentKey, RequestCancel>,
    pub cancels_refused:
        NoneOneOrMany<RiskRefused<Order<ExchangeKey, InstrumentKey, RequestCancel>>>,
    pub opens: SendRequestsOutput<ExchangeKey, InstrumentKey, RequestOpen>,
    pub opens_refused: NoneOneOrMany<RiskRefused<Order<ExchangeKey, InstrumentKey, RequestOpen>>>,
}

impl<ExchangeKey, InstrumentKey> GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey> {
    pub fn unrecoverable_errors(&self) -> Option<OneOrMany<UnrecoverableEngineError>> {
        match (
            self.cancels.unrecoverable_errors(),
            self.opens.unrecoverable_errors(),
        ) {
            (None, None) => None,
            (Some(cancels), Some(opens)) => Some(cancels.into_iter().chain(opens).collect()),
            (Some(cancels), None) => Some(cancels),
            (None, Some(opens)) => Some(opens),
        }
    }
}

impl<MarketState, Strategy, Risk, ExecutionTxs, ExchangeKey, AssetKey, InstrumentKey>
    GenerateAlgoOrders<ExchangeKey, InstrumentKey>
    for Engine<
        EngineState<
            MarketState,
            Strategy::State,
            Risk::State,
            ExchangeKey,
            AssetKey,
            InstrumentKey,
        >,
        ExecutionTxs,
        Strategy,
        Risk,
    >
where
    ExecutionTxs: ExecutionTxMap<ExchangeKey, InstrumentKey>,
    Strategy: AlgoStrategy<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
    Risk: RiskManager<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
    ExchangeKey: Debug + Clone,
    InstrumentKey: Debug + Clone,
{
    fn generate_algo_orders(&mut self) -> GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey> {
        // Generate orders
        let (cancels, opens) = self.strategy.generate_algo_orders(
            &self.state.strategy,
            &self.state.assets,
            &self.state.instruments,
        );

        // RiskApprove & RiskRefuse order requests
        let (cancels, opens, refused_cancels, refused_opens) = self.risk.check(
            &self.state.risk,
            &self.state.assets,
            &self.state.instruments,
            cancels,
            opens,
        );

        // Send risk approved order requests
        let cancels = self.send_requests(cancels.into_iter().map(|RiskApproved(cancel)| cancel));
        let opens = self.send_requests(opens.into_iter().map(|RiskApproved(open)| open));

        // Collect remaining Iterators (so we can access &mut self)
        let cancels_refused = refused_cancels.into_iter().collect();
        let opens_refused = refused_opens.into_iter().collect();

        // Record in flight order requests
        self.state.record_in_flight_cancels(cancels.sent.iter());
        self.state.record_in_flight_opens(opens.sent.iter());

        GenerateAlgoOrdersOutput::new(cancels, cancels_refused, opens, opens_refused)
    }
}
