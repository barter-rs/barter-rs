use crate::engine::state::order::in_flight_recorder::InFlightRequestRecorder;
use barter_execution::order::{
    Order,
    request::OrderResponseCancel,
    state::{ActiveOrderState, OrderState},
};
use barter_integration::snapshot::Snapshot;
use std::fmt::Debug;

/// Synchronous order manager that tracks the lifecycle of active exchange orders.
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

    fn update_from_order_snapshot<AssetKey>(
        &mut self,
        snapshot: Snapshot<&Order<ExchangeKey, InstrumentKey, OrderState<AssetKey, InstrumentKey>>>,
    ) where
        AssetKey: Debug + Clone;

    fn update_from_cancel_response<AssetKey>(
        &mut self,
        response: &OrderResponseCancel<ExchangeKey, AssetKey, InstrumentKey>,
    ) where
        AssetKey: Debug + Clone;
}
