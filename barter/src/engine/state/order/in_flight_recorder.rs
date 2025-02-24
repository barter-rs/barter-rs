use crate::engine::state::EngineState;
use barter_execution::order::request::{OrderRequestCancel, OrderRequestOpen};
use barter_instrument::{exchange::ExchangeIndex, instrument::InstrumentIndex};

/// Synchronous in-flight open and in-flight cancel order request tracker.
///
/// See [`Orders`](super::Orders) for an example implementation.
pub trait InFlightRequestRecorder<ExchangeKey, InstrumentKey> {
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

impl<Market, Strategy, Risk> InFlightRequestRecorder<ExchangeIndex, InstrumentIndex>
    for EngineState<Market, Strategy, Risk>
{
    fn record_in_flight_cancel(
        &mut self,
        request: &OrderRequestCancel<ExchangeIndex, InstrumentIndex>,
    ) {
        self.instruments
            .instrument_index_mut(&request.key.instrument)
            .orders
            .record_in_flight_cancel(request);
    }

    fn record_in_flight_open(
        &mut self,
        request: &OrderRequestOpen<ExchangeIndex, InstrumentIndex>,
    ) {
        self.instruments
            .instrument_index_mut(&request.key.instrument)
            .orders
            .record_in_flight_open(request);
    }
}
