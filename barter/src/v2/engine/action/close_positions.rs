use crate::v2::{
    engine::{
        action::send_requests::SendRequestsOutput,
        execution_tx::ExecutionTxMap,
        state::{
            instrument::manager::{InstrumentFilter, InstrumentStateManager},
            order::in_flight_recorder::InFlightRequestRecorder,
            EngineState,
        },
        Engine,
    },
    order::{RequestCancel, RequestOpen},
    risk::RiskManager,
    strategy::close_positions::ClosePositionsStrategy,
};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub trait ClosePositions<ExchangeKey, AssetKey, InstrumentKey> {
    fn close_positions(
        &mut self,
        filter: &InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>,
    ) -> ClosePositionsOutput<ExchangeKey, InstrumentKey>;
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct ClosePositionsOutput<ExchangeKey, InstrumentKey> {
    pub cancels: SendRequestsOutput<ExchangeKey, InstrumentKey, RequestCancel>,
    pub opens: SendRequestsOutput<ExchangeKey, InstrumentKey, RequestOpen>,
}

impl<MarketState, Strategy, Risk, ExecutionTxs, ExchangeKey, AssetKey, InstrumentKey>
    ClosePositions<ExchangeKey, AssetKey, InstrumentKey>
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
    EngineState<MarketState, Strategy::State, Risk::State, ExchangeKey, AssetKey, InstrumentKey>:
        InstrumentStateManager<InstrumentKey, ExchangeKey = ExchangeKey>,
    ExecutionTxs: ExecutionTxMap<ExchangeKey, InstrumentKey>,
    Strategy: ClosePositionsStrategy<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
    Risk: RiskManager<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
    ExchangeKey: Debug + Clone,
    InstrumentKey: Debug + Clone,
{
    fn close_positions(
        &mut self,
        filter: &InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>,
    ) -> ClosePositionsOutput<ExchangeKey, InstrumentKey> {
        // Generate orders
        let (cancels, opens) = self.strategy.close_positions_requests(
            &self.state.strategy,
            &self.state.assets,
            &self.state.instruments,
            filter,
        );

        // Bypass risk checks...

        // Send order requests
        let cancels = self.send_requests(cancels);
        let opens = self.send_requests(opens);

        // Record in flight order requests
        self.state.record_in_flight_cancels(&cancels.sent);
        self.state.record_in_flight_opens(&opens.sent);

        ClosePositionsOutput::new(cancels, opens)
    }
}
