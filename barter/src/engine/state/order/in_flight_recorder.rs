use crate::engine::state::EngineState;
use barter_execution::order::request::{OrderRequestCancel, OrderRequestOpen};
use barter_instrument::{exchange::ExchangeIndex, instrument::InstrumentIndex};

/// Synchronous in-flight open and in-flight cancel order request tracker.
///
/// See [`Orders`](super::Orders) for an example implementation.
pub trait InFlightRequestRecorder<ExchangeKey = ExchangeIndex, InstrumentKey = InstrumentIndex> {
    fn record_in_flight_cancels<'a>(
        &mut self,
        requests: impl IntoIterator<Item = &'a OrderRequestCancel<ExchangeKey, InstrumentKey>>,
    ) where
        ExchangeKey: 'a,
        InstrumentKey: 'a,
    {
        requests
            .into_iter()
            .for_each(|request| self.record_in_flight_cancel(request))
    }

    fn record_in_flight_opens<'a>(
        &mut self,
        requests: impl IntoIterator<Item = &'a OrderRequestOpen<ExchangeKey, InstrumentKey>>,
    ) where
        ExchangeKey: 'a,
        InstrumentKey: 'a,
    {
        requests
            .into_iter()
            .for_each(|request| self.record_in_flight_open(request))
    }

    fn record_in_flight_cancel(&mut self, request: &OrderRequestCancel<ExchangeKey, InstrumentKey>);

    fn record_in_flight_open(&mut self, request: &OrderRequestOpen<ExchangeKey, InstrumentKey>);
}

impl<GlobalData, InstrumentData> InFlightRequestRecorder<ExchangeIndex, InstrumentIndex>
    for EngineState<GlobalData, InstrumentData>
where
    InstrumentData: InFlightRequestRecorder<ExchangeIndex, InstrumentIndex>,
{
    fn record_in_flight_cancel(
        &mut self,
        request: &OrderRequestCancel<ExchangeIndex, InstrumentIndex>,
    ) {
        let instrument_state = self
            .instruments
            .instrument_index_mut(&request.key.instrument);

        instrument_state.orders.record_in_flight_cancel(request);
        instrument_state.data.record_in_flight_cancel(request);
    }

    fn record_in_flight_open(
        &mut self,
        request: &OrderRequestOpen<ExchangeIndex, InstrumentIndex>,
    ) {
        let instrument_state = self
            .instruments
            .instrument_index_mut(&request.key.instrument);

        instrument_state.orders.record_in_flight_open(request);
        instrument_state.data.record_in_flight_open(request);
    }
}
