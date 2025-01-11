use crate::engine::state::order::in_flight_recorder::InFlightRequestRecorder;
use barter_execution::{
    error::ClientError,
    order::{
        state::{ActiveOrderState, Cancelled, Open, OrderState},
        Order,
    },
};
use barter_integration::snapshot::Snapshot;
use std::fmt::Debug;

/// Synchronous order manager that tracks the lifecycle of exchange orders.
///
/// See [`Orders`](super::Orders) for an example implementation.
pub trait OrderManager<ExchangeKey, InstrumentKey>
where
    Self: InFlightRequestRecorder<ExchangeKey, InstrumentKey>,
{
    fn orders<'a>(
        &'a self,
    ) -> impl Iterator<Item = &'a Order<ExchangeKey, InstrumentKey, ActiveOrderState>>
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
        snapshot: Snapshot<&Order<ExchangeKey, InstrumentKey, OrderState>>,
    );
}
