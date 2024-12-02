use crate::v2::{
    engine::state::order::in_flight_recorder::InFlightRequestRecorder,
    execution::error::ClientError,
    order::{Cancelled, ExchangeOrderState, InternalOrderState, Open, Order},
    Snapshot,
};
use std::fmt::Debug;

pub trait OrderManager<ExchangeKey, InstrumentKey>
where
    Self: InFlightRequestRecorder<ExchangeKey, InstrumentKey>,
{
    fn orders<'a>(
        &'a self,
    ) -> impl Iterator<Item = &'a Order<ExchangeKey, InstrumentKey, InternalOrderState>>
    where
        ExchangeKey: 'a,
        InstrumentKey: 'a;

    fn update_from_open<AssetKey>(
        &mut self,
        response: &Order<
            ExchangeKey,
            InstrumentKey,
            Result<Open, ClientError<AssetKey, InstrumentKey>>,
        >,
    ) where
        AssetKey: Debug;

    fn update_from_cancel<AssetKey>(
        &mut self,
        response: &Order<
            ExchangeKey,
            InstrumentKey,
            Result<Cancelled, ClientError<AssetKey, InstrumentKey>>,
        >,
    ) where
        AssetKey: Debug;

    fn update_from_order_snapshot(
        &mut self,
        snapshot: Snapshot<&Order<ExchangeKey, InstrumentKey, ExchangeOrderState>>,
    );
}
