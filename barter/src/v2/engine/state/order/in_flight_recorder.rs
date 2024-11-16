use crate::v2::order::{Order, RequestCancel, RequestOpen};

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
