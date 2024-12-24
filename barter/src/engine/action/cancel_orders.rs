use crate::engine::{
    action::send_requests::{SendRequests, SendRequestsOutput},
    execution_tx::ExecutionTxMap,
    state::{
        instrument::manager::InstrumentFilter,
        order::{in_flight_recorder::InFlightRequestRecorder, manager::OrderManager},
        EngineState,
    },
    Engine,
};
use barter_execution::order::{Order, RequestCancel};
use barter_instrument::{asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex};

pub trait CancelOrders<
    ExchangeKey = ExchangeIndex,
    AssetKey = AssetIndex,
    InstrumentKey = InstrumentIndex,
>
{
    type Output;

    fn cancel_orders(
        &mut self,
        filter: &InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>,
    ) -> Self::Output;
}

impl<MarketState, StrategyState, RiskState, ExecutionTxs, Strategy, Risk> CancelOrders
    for Engine<EngineState<MarketState, StrategyState, RiskState>, ExecutionTxs, Strategy, Risk>
where
    ExecutionTxs: ExecutionTxMap,
{
    type Output = SendRequestsOutput<ExchangeIndex, InstrumentIndex, RequestCancel>;

    fn cancel_orders(
        &mut self,
        filter: &InstrumentFilter<ExchangeIndex, AssetIndex, InstrumentIndex>,
    ) -> Self::Output {
        let requests = self
            .state
            .instruments
            .filtered(filter)
            .flat_map(|state| state.orders.orders().filter_map(Order::as_request_cancel));

        // Bypass risk checks...

        // Send order requests
        let cancels = self.send_requests(requests);

        // Record in flight order requests
        self.state.record_in_flight_cancels(&cancels.sent);

        cancels
    }
}
