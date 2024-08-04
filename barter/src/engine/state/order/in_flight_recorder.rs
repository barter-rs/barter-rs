use crate::engine::state::EngineState;
use barter_execution::order::{Order, RequestCancel, RequestOpen};
use barter_instrument::{exchange::ExchangeIndex, instrument::InstrumentIndex};

/// Synchronous in-flight open and in-flight cancel order request tracker.
///
/// See [`Orders`](super::Orders) for an example implementation.
pub trait InFlightRequestRecorder<ExchangeKey, InstrumentKey> {
    fn record_in_flight_cancels<'a>(
        &mut self,
        requests: impl IntoIterator<Item = &'a Order<ExchangeKey, InstrumentKey, RequestCancel>>,
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
        requests: impl IntoIterator<Item = &'a Order<ExchangeKey, InstrumentKey, RequestOpen>>,
    ) where
        ExchangeKey: 'a,
        InstrumentKey: 'a,
    {
        requests
            .into_iter()
            .for_each(|request| self.record_in_flight_open(request))
    }

    fn record_in_flight_cancel(
        &mut self,
        request: &Order<ExchangeKey, InstrumentKey, RequestCancel>,
    );

    fn record_in_flight_open(&mut self, request: &Order<ExchangeKey, InstrumentKey, RequestOpen>);
}

impl<Market, Strategy, Risk> InFlightRequestRecorder<ExchangeIndex, InstrumentIndex>
    for EngineState<Market, Strategy, Risk>
{
    fn record_in_flight_cancel(
        &mut self,
        request: &Order<ExchangeIndex, InstrumentIndex, RequestCancel>,
    ) {
        self.instruments
            .instrument_index_mut(&request.instrument)
            .orders
            .record_in_flight_cancel(request);
    }

    fn record_in_flight_open(
        &mut self,
        request: &Order<ExchangeIndex, InstrumentIndex, RequestOpen>,
    ) {
        self.instruments
            .instrument_index_mut(&request.instrument)
            .orders
            .record_in_flight_open(request);
    }
}
