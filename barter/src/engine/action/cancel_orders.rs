use crate::engine::{
    action::send_requests::{SendRequests, SendRequestsOutput},
    execution_tx::ExecutionTxMap,
    state::{
        instrument::filter::InstrumentFilter,
        order::{in_flight_recorder::InFlightRequestRecorder, manager::OrderManager},
        EngineState,
    },
    Engine,
};
use barter_execution::order::{request::RequestCancel, Order};
use barter_instrument::{asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex};

/// Trait that defines how the [`Engine`] cancels open order requests.
///
/// # Type Parameters
/// * `ExchangeKey` - Type used to identify an exchange (defaults to [`ExchangeIndex`]).
/// * `AssetKey` - Type used to identify an asset (defaults to [`AssetIndex`]).
/// * `InstrumentKey` - Type used to identify an instrument (defaults to [`InstrumentIndex`]).
pub trait CancelOrders<
    ExchangeKey = ExchangeIndex,
    AssetKey = AssetIndex,
    InstrumentKey = InstrumentIndex,
>
{
    /// Generates cancel order requests.
    ///
    /// Uses the provided [`InstrumentFilter`] to determine which orders to cancel.
    fn cancel_orders(
        &mut self,
        filter: &InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>,
    ) -> SendRequestsOutput<RequestCancel, ExchangeKey, InstrumentKey>;
}

impl<Clock, MarketState, StrategyState, RiskState, ExecutionTxs, Strategy, Risk> CancelOrders
    for Engine<
        Clock,
        EngineState<MarketState, StrategyState, RiskState>,
        ExecutionTxs,
        Strategy,
        Risk,
    >
where
    ExecutionTxs: ExecutionTxMap,
{
    fn cancel_orders(
        &mut self,
        filter: &InstrumentFilter<ExchangeIndex, AssetIndex, InstrumentIndex>,
    ) -> SendRequestsOutput<RequestCancel, ExchangeIndex, InstrumentIndex> {
        let requests = self
            .state
            .instruments
            .orders(filter)
            .flat_map(|state| state.orders().filter_map(Order::to_request_cancel));

        // Bypass risk checks...

        // Send order requests
        let cancels = self.send_requests(requests);

        // Record in flight order requests
        self.state.record_in_flight_cancels(&cancels.sent);

        cancels
    }
}
