use crate::{
    engine::{
        action::send_requests::{SendCancelsAndOpensOutput, SendRequests},
        execution_tx::ExecutionTxMap,
        state::{
            instrument::manager::InstrumentFilter,
            order::in_flight_recorder::InFlightRequestRecorder,
        },
        Engine,
    },
    strategy::close_positions::ClosePositionsStrategy,
};
use barter_instrument::{asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex};
use std::fmt::Debug;

pub trait ClosePositions<
    ExchangeKey = ExchangeIndex,
    AssetKey = AssetIndex,
    InstrumentKey = InstrumentIndex,
>
{
    fn close_positions(
        &mut self,
        filter: &InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>,
    ) -> SendCancelsAndOpensOutput<ExchangeKey, InstrumentKey>;
}

impl<State, ExecutionTxs, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey>
    ClosePositions<ExchangeKey, AssetKey, InstrumentKey>
    for Engine<State, ExecutionTxs, Strategy, Risk>
where
    State: InFlightRequestRecorder<ExchangeKey, InstrumentKey>,
    ExecutionTxs: ExecutionTxMap<ExchangeKey, InstrumentKey>,
    Strategy: ClosePositionsStrategy<ExchangeKey, AssetKey, InstrumentKey, State = State>,
    ExchangeKey: Debug + Clone,
    InstrumentKey: Debug + Clone,
{
    fn close_positions(
        &mut self,
        filter: &InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>,
    ) -> SendCancelsAndOpensOutput<ExchangeKey, InstrumentKey> {
        // Generate orders
        let (cancels, opens) = self.strategy.close_positions_requests(&self.state, filter);

        // Bypass risk checks...

        // Send order requests
        let cancels = self.send_requests(cancels);
        let opens = self.send_requests(opens);

        // Record in flight order requests
        self.state.record_in_flight_cancels(&cancels.sent);
        self.state.record_in_flight_opens(&opens.sent);

        SendCancelsAndOpensOutput::new(cancels, opens)
    }
}
