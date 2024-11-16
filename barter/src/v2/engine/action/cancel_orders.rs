use crate::v2::{
    engine::{
        action::send_requests::{SendRequests, SendRequestsOutput},
        execution_tx::ExecutionTxMap,
        state::{
            instrument::manager::{InstrumentFilter, InstrumentStateManager},
            order::{in_flight_recorder::InFlightRequestRecorder, manager::OrderManager},
        },
        Engine,
    },
    order::{Order, RequestCancel},
};
use std::fmt::Debug;

pub trait CancelOrders<ExchangeKey, AssetKey, InstrumentKey> {
    type Output;

    fn cancel_orders(
        &mut self,
        filter: &InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>,
    ) -> Self::Output;
}

impl<State, ExecutionTxs, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey>
    CancelOrders<ExchangeKey, AssetKey, InstrumentKey>
    for Engine<State, ExecutionTxs, Strategy, Risk>
where
    State: InstrumentStateManager<InstrumentKey, ExchangeKey = ExchangeKey, AssetKey = AssetKey>
        + InFlightRequestRecorder<ExchangeKey, InstrumentKey>,
    ExecutionTxs: ExecutionTxMap<ExchangeKey, InstrumentKey>,
    ExchangeKey: Debug + Clone + PartialEq,
    AssetKey: PartialEq,
    InstrumentKey: Debug + Clone + PartialEq,
{
    type Output = SendRequestsOutput<ExchangeKey, InstrumentKey, RequestCancel>;

    fn cancel_orders(
        &mut self,
        filter: &InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>,
    ) -> Self::Output {
        let requests = self
            .state
            .instruments_filtered(filter)
            .flat_map(|state| state.orders.orders().filter_map(Order::as_request_cancel));

        // Bypass risk checks...

        // Send order requests
        let cancels = self.send_requests(requests);

        // Record in flight order requests
        self.state.record_in_flight_cancels(&cancels.sent);

        cancels
    }
}
