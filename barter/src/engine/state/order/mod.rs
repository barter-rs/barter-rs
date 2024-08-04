use crate::engine::state::order::{
    in_flight_recorder::InFlightRequestRecorder, manager::OrderManager,
};
use barter_execution::{
    error::{ApiError, ClientError},
    order::{
        Cancelled, ClientOrderId, ExchangeOrderState, InternalOrderState, Open, Order,
        RequestCancel, RequestOpen,
    },
};
use barter_integration::snapshot::Snapshot;
use derive_more::Constructor;
use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};
use std::{collections::hash_map::Entry, fmt::Debug};
use tracing::{debug, error, warn};

pub mod in_flight_recorder;
pub mod manager;

/// Synchronous order manager that tracks the lifecycle of exchange orders.
///
/// The `Orders` struct maintains a `FnvHashMap` of orders keyed by their [`ClientOrderId`].
///
/// Implements the [`OrderManager`] and [`InFlightRequestRecorder`] traits.
///
/// A distinct instance of `Orders` is used in the engine
/// [`InstrumentState`](super::instrument::InstrumentState) to track the active orders for
/// each instrument, however it could be used to track global orders if [`ClientOrderId`]
/// is globally unique.
///
/// # State Transitions
/// Orders progress through the following states:
/// 1. OpenInFlight - Initial order request sent to exchange
/// 2. Open - Order confirmed as open on exchange
/// 3. CancelInFlight - Cancellation request sent to exchange
/// 4. Cancelled/Expired/FullyFilled - Terminal states, once achieved order is no longer tracked.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct Orders<ExchangeKey, InstrumentKey>(
    pub FnvHashMap<ClientOrderId, Order<ExchangeKey, InstrumentKey, InternalOrderState>>,
);

impl<ExchangeKey, InstrumentKey> Default for Orders<ExchangeKey, InstrumentKey> {
    fn default() -> Self {
        Self(FnvHashMap::default())
    }
}

impl<ExchangeKey, InstrumentKey> OrderManager<ExchangeKey, InstrumentKey>
    for Orders<ExchangeKey, InstrumentKey>
where
    ExchangeKey: Debug + Clone,
    InstrumentKey: Debug + Clone,
{
    fn orders<'a>(
        &'a self,
    ) -> impl Iterator<Item = &'a Order<ExchangeKey, InstrumentKey, InternalOrderState>>
    where
        ExchangeKey: 'a,
        InstrumentKey: 'a,
    {
        self.0.values()
    }

    fn update_from_open<AssetKey>(
        &mut self,
        response: &Order<
            ExchangeKey,
            InstrumentKey,
            Result<Open, ClientError<AssetKey, InstrumentKey>>,
        >,
    ) where
        AssetKey: Debug,
    {
        match (self.0.entry(response.cid.clone()), &response.state) {
            (Entry::Occupied(mut order), Ok(new_open)) => match &order.get().state {
                InternalOrderState::OpenInFlight(_) => {
                    debug!(
                        exchange = ?response.exchange,
                        instrument = ?response.instrument,
                        cid = %response.cid,
                        order = ?order.get(),
                        update = ?response,
                        "OrderManager transitioned Order<OpenInFlight> to Order<Open>"
                    );
                    // If Order was OrderKind::Market it may be fully filled
                    if new_open.quantity_remaining().is_zero() {
                        order.remove();
                    } else {
                        order.get_mut().state = InternalOrderState::Open(new_open.clone());
                    }
                }
                InternalOrderState::Open(existing_open) => {
                    warn!(
                        exchange = ?response.exchange,
                        instrument = ?response.instrument,
                        cid = %response.cid,
                        order = ?order.get(),
                        update = ?response,
                        "OrderManager received Order<Open> Ok response for existing Order<Open> - taking latest timestamp"
                    );

                    if new_open.time_exchange > existing_open.time_exchange {
                        order.get_mut().state = InternalOrderState::Open(new_open.clone());
                    }
                }
                InternalOrderState::CancelInFlight(_) => {
                    warn!(
                        exchange = ?response.exchange,
                        instrument = ?response.instrument,
                        cid = %response.cid,
                        order = ?order.get(),
                        update = ?response,
                        "OrderManager received Order<Open> Ok response for existing Order<CancelInFlight>"
                    );
                }
            },
            (Entry::Vacant(cid_untracked), Ok(new_open)) => {
                warn!(
                    exchange = ?response.exchange,
                    instrument = ?response.instrument,
                    cid = %response.cid,
                    update = ?response,
                    "OrderManager received Order<Open> for untracked ClientOrderId - now tracking"
                );

                cid_untracked.insert(Order::new(
                    response.exchange.clone(),
                    response.instrument.clone(),
                    response.strategy.clone(),
                    response.cid.clone(),
                    response.side,
                    InternalOrderState::from(new_open.clone()),
                ));
            }
            (Entry::Occupied(order), Err(error)) => match &order.get().state {
                InternalOrderState::OpenInFlight(_) => {
                    warn!(
                        exchange = ?response.exchange,
                        instrument = ?response.instrument,
                        cid = %response.cid,
                        order = ?order.get(),
                        update = ?response,
                        "OrderManager received ClientError for Order<OpenInFlight>"
                    );
                    order.remove();
                }
                InternalOrderState::Open(_) => {
                    if matches!(
                        error,
                        ClientError::Api(ApiError::OrderAlreadyCancelled)
                            | ClientError::Api(ApiError::OrderAlreadyFullyFilled)
                    ) {
                        debug!(
                            exchange = ?response.exchange,
                            instrument = ?response.instrument,
                            cid = %response.cid,
                            order = ?order.get(),
                            update = ?response,
                            "OrderManager received 'order already completed' for Order<Open> - removing"
                        );
                        order.remove();
                    } else {
                        error!(
                            exchange = ?response.exchange,
                            instrument = ?response.instrument,
                            cid = %response.cid,
                            order = ?order.get(),
                            update = ?response,
                            "OrderManager received ClientError for existing Order<Open>"
                        );
                    }
                }
                InternalOrderState::CancelInFlight(_) => {
                    if matches!(
                        error,
                        ClientError::Api(ApiError::OrderAlreadyCancelled)
                            | ClientError::Api(ApiError::OrderAlreadyFullyFilled)
                    ) {
                        warn!(
                            exchange = ?response.exchange,
                            instrument = ?response.instrument,
                            cid = %response.cid,
                            order = ?order.get(),
                            update = ?response,
                            "OrderManager received 'order already completed' for Order<CancelInFlight> - removing"
                        );
                        order.remove();
                    } else {
                        error!(
                            exchange = ?response.exchange,
                            instrument = ?response.instrument,
                            cid = %response.cid,
                            ?order,
                            update = ?response,
                            "OrderManager received ClientError for existing Order<CancelInFlight>"
                        );
                    }
                }
            },
            (Entry::Vacant(_), Err(_)) => {
                error!(
                    exchange = ?response.exchange,
                    instrument = ?response.instrument,
                    cid = %response.cid,
                    update = ?response,
                    "OrderManager received ExecutionError for untracked ClientOrderId"
                );
            }
        }
    }

    fn update_from_cancel<AssetKey>(
        &mut self,
        response: &Order<
            ExchangeKey,
            InstrumentKey,
            Result<Cancelled, ClientError<AssetKey, InstrumentKey>>,
        >,
    ) where
        AssetKey: Debug,
    {
        match (self.0.entry(response.cid.clone()), &response.state) {
            (Entry::Occupied(order), Ok(_new_cancel)) => match &order.get().state {
                InternalOrderState::OpenInFlight(_) => {
                    debug!(
                        exchange = ?response.exchange,
                        instrument = ?response.instrument,
                        cid = %response.cid,
                        order = ?order.get(),
                        update = ?response,
                        "OrderManager transitioned Order<OpenInFlight> to Order<Cancelled>"
                    );
                    order.remove();
                }
                InternalOrderState::Open(_) => {
                    warn!(
                        exchange = ?response.exchange,
                        instrument = ?response.instrument,
                        cid = %response.cid,
                        order = ?order.get(),
                        update = ?response,
                        "OrderManager received Order<Cancelled> Ok response for existing Order<Open>"
                    );
                    order.remove();
                }
                InternalOrderState::CancelInFlight(_) => {
                    debug!(
                        exchange = ?response.exchange,
                        instrument = ?response.instrument,
                        cid = %response.cid,
                        order = ?order.get(),
                        update = ?response,
                        "OrderManager transitioned Order<CancelInFlight> to Order<Cancelled>"
                    );
                    order.remove();
                }
            },
            (Entry::Vacant(_cid_untracked), Ok(_new_cancel)) => {
                warn!(
                    exchange = ?response.exchange,
                    instrument = ?response.instrument,
                    cid = %response.cid,
                    update = ?response,
                    "OrderManager received Order<Cancelled> Ok response for untracked ClientOrderId - ignoring"
                );
            }
            (Entry::Occupied(order), Err(error)) => match &order.get().state {
                InternalOrderState::OpenInFlight(_) => {
                    error!(
                        exchange = ?response.exchange,
                        instrument = ?response.instrument,
                        cid = %response.cid,
                        order = ?order.get(),
                        update = ?response,
                        "OrderManager received ClientError for Order<OpenInFlight> whilst cancelling"
                    );

                    order.remove();
                }
                InternalOrderState::Open(_) => {
                    if matches!(
                        error,
                        ClientError::Api(ApiError::OrderAlreadyCancelled)
                            | ClientError::Api(ApiError::OrderAlreadyFullyFilled)
                    ) {
                        debug!(
                            exchange = ?response.exchange,
                            instrument = ?response.instrument,
                            cid = %response.cid,
                            order = ?order.get(),
                            update = ?response,
                            "OrderManager received 'order already completed' for Order<CancelInFlight> - removing"
                        );
                        order.remove();
                    } else {
                        error!(
                            exchange = ?response.exchange,
                            instrument = ?response.instrument,
                            cid = %response.cid,
                            order = ?order.get(),
                            update = ?response,
                            "OrderManager received ClientError for Order<Open> whilst cancelling"
                        );
                    }
                }
                InternalOrderState::CancelInFlight(_) => {
                    error!(
                        instrument = ?response.instrument,
                        cid = %response.cid,
                        update = ?response,
                        "OrderManager received ClientError for Order<CancelInFlight> - removing"
                    );
                    order.remove();
                }
            },
            (Entry::Vacant(_), Err(_)) => {
                error!(
                    instrument = ?response.instrument,
                    cid = %response.cid,
                    update = ?response,
                    "OrderManager received ExecutionError for untracked ClientOrderId"
                );
            }
        }
    }

    fn update_from_order_snapshot(
        &mut self,
        snapshot: Snapshot<&Order<ExchangeKey, InstrumentKey, ExchangeOrderState>>,
    ) {
        match self.0.entry(snapshot.0.cid.clone()) {
            Entry::Occupied(mut order) => match &snapshot.0.state {
                ExchangeOrderState::Cancelled(_) => {
                    debug!(
                        exchange = ?snapshot.0.exchange,
                        instrument = ?snapshot.0.instrument,
                        cid = %snapshot.0.cid,
                        order = ?order.get(),
                        update = ?snapshot,
                        "OrderManager received Snapshot<Order<Cancelled>> for tracked Order - removing"
                    );
                    order.remove();
                }
                ExchangeOrderState::FullyFilled => match &order.get().state {
                    InternalOrderState::OpenInFlight(_) | InternalOrderState::Open(_) => {
                        debug!(
                            exchange = ?snapshot.0.exchange,
                            instrument = ?snapshot.0.instrument,
                            cid = %snapshot.0.cid,
                            order = ?order.get(),
                            update = ?snapshot,
                            "OrderManager received Snapshot<Order<Filled>> for tracked Order - removing"
                        );
                        order.remove();
                    }
                    InternalOrderState::CancelInFlight(_) => {
                        warn!(
                            exchange = ?snapshot.0.exchange,
                            instrument = ?snapshot.0.instrument,
                            cid = %snapshot.0.cid,
                            order = ?order.get(),
                            update = ?snapshot,
                            "OrderManager received Snapshot<Order<Filled>> for Order<CancelInFlight> - removing"
                        );
                        order.remove();
                    }
                },
                ExchangeOrderState::Expired => {
                    debug!(
                        exchange = ?snapshot.0.exchange,
                        instrument = ?snapshot.0.instrument,
                        cid = %snapshot.0.cid,
                        order = ?order.get(),
                        update = ?snapshot,
                        "OrderManager received Snapshot<Order<Expired>> for tracked Order - removing"
                    );
                    order.remove();
                }
                ExchangeOrderState::Rejected(reason) => {
                    warn!(
                        exchange = ?snapshot.0.exchange,
                        instrument = ?snapshot.0.instrument,
                        cid = %snapshot.0.cid,
                        order = ?order.get(),
                        update = ?snapshot,
                        ?reason,
                        "OrderManager received Snapshot<Order<Rejected>> for Order<CancelInFlight> - removing"
                    );
                    order.remove();
                }
                ExchangeOrderState::Open(exchange_open) => match &order.get().state {
                    InternalOrderState::OpenInFlight(_) => {
                        debug!(
                            exchange = ?snapshot.0.exchange,
                            instrument = ?snapshot.0.instrument,
                            cid = %snapshot.0.cid,
                            order = ?order.get(),
                            update = ?snapshot,
                            "OrderManager transitioned Order<OpenInFlight> to Order<Open>"
                        );
                        order.get_mut().state = InternalOrderState::Open(exchange_open.clone())
                    }
                    InternalOrderState::Open(internal) => {
                        debug!(
                            exchange = ?snapshot.0.exchange,
                            instrument = ?snapshot.0.instrument,
                            cid = %snapshot.0.cid,
                            order = ?order.get(),
                            update = ?snapshot,
                            "OrderManager received Snapshot<Order<Open>> for existing Order<Open> - taking latest timestamp"
                        );

                        if internal.time_exchange < exchange_open.time_exchange {
                            order.get_mut().state = InternalOrderState::Open(exchange_open.clone())
                        }
                    }
                    InternalOrderState::CancelInFlight(_) => {
                        // Waiting for cancel acknowledge, so do nothing
                        debug!(
                            exchange = ?snapshot.0.exchange,
                            instrument = ?snapshot.0.instrument,
                            cid = %snapshot.0.cid,
                            order = ?order.get(),
                            update = ?snapshot,
                            "OrderManager received Snapshot<Order<Open>> for existing Order<CancelInFlight> - ignoring"
                        );
                    }
                },
            },
            Entry::Vacant(untracked_cid) => match &snapshot.0.state {
                ExchangeOrderState::Cancelled(_) => {
                    // Order untracked, so ignore cancel acknowledgement
                    warn!(
                        exchange = ?snapshot.0.exchange,
                        instrument = ?snapshot.0.instrument,
                        cid = %snapshot.0.cid,
                        update = ?snapshot,
                        "OrderManager received Snapshot<Order<Cancelled>> for untracked Order - ignoring"
                    );
                }
                ExchangeOrderState::FullyFilled => {
                    warn!(
                        exchange = ?snapshot.0.exchange,
                        instrument = ?snapshot.0.instrument,
                        cid = %snapshot.0.cid,
                        update = ?snapshot,
                        "OrderManager received Snapshot<Order<Filled>> for untracked Order - ignoring"
                    );
                }
                ExchangeOrderState::Expired => {
                    warn!(
                        exchange = ?snapshot.0.exchange,
                        instrument = ?snapshot.0.instrument,
                        cid = %snapshot.0.cid,
                        update = ?snapshot,
                        "OrderManager received Snapshot<Order<Expired>> for untracked Order - ignoring"
                    );
                }
                ExchangeOrderState::Rejected(reason) => {
                    warn!(
                        exchange = ?snapshot.0.exchange,
                        instrument = ?snapshot.0.instrument,
                        cid = %snapshot.0.cid,
                        update = ?snapshot,
                        ?reason,
                        "OrderManager received Snapshot<Order<Rejected>> for untracked Order - ignoring"
                    );
                }
                ExchangeOrderState::Open(exchange_open) => {
                    warn!(
                        exchange = ?snapshot.0.exchange,
                        instrument = ?snapshot.0.instrument,
                        cid = %snapshot.0.cid,
                        update = ?snapshot,
                        "OrderManager received Snapshot<Order<Open>> for untracked Order - now tracking",
                    );

                    untracked_cid.insert(Order::new(
                        snapshot.0.exchange.clone(),
                        snapshot.0.instrument.clone(),
                        snapshot.0.strategy.clone(),
                        snapshot.0.cid.clone(),
                        snapshot.0.side,
                        InternalOrderState::Open(exchange_open.clone()),
                    ));
                }
            },
        }
    }
}

impl<ExchangeKey, InstrumentKey> InFlightRequestRecorder<ExchangeKey, InstrumentKey>
    for Orders<ExchangeKey, InstrumentKey>
where
    ExchangeKey: Debug + Clone,
    InstrumentKey: Debug + Clone,
{
    fn record_in_flight_cancel(
        &mut self,
        request: &Order<ExchangeKey, InstrumentKey, RequestCancel>,
    ) {
        if let Some(duplicate_cid_order) = self.0.insert(request.cid.clone(), Order::from(request))
        {
            error!(
                cid = %duplicate_cid_order.cid,
                event = ?duplicate_cid_order,
                "OrderManager upserted Order CancelInFlight with duplicate ClientOrderId"
            );
        }
    }

    fn record_in_flight_open(&mut self, request: &Order<ExchangeKey, InstrumentKey, RequestOpen>) {
        if let Some(duplicate_cid_order) = self.0.insert(request.cid.clone(), Order::from(request))
        {
            error!(
                cid = %duplicate_cid_order.cid,
                event = ?duplicate_cid_order,
                "OrderManager upserted Order OpenInFlight with duplicate ClientOrderId"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::state::order::Orders;
    use barter_execution::{
        error::ConnectivityError,
        order::{
            CancelInFlight, ClientOrderId, InternalOrderState, OpenInFlight, Order, OrderId,
            OrderKind, RequestOpen, StrategyId, TimeInForce,
        },
    };
    use barter_instrument::{exchange::ExchangeId, Side};
    use chrono::{DateTime, Utc};
    use rust_decimal_macros::dec;
    use smol_str::SmolStr;

    fn orders(
        orders: impl IntoIterator<Item = Order<ExchangeId, u64, InternalOrderState>>,
    ) -> Orders<ExchangeId, u64> {
        Orders(
            orders
                .into_iter()
                .map(|order| (order.cid.clone(), order))
                .collect(),
        )
    }

    fn order<State>(cid: ClientOrderId, state: State) -> Order<ExchangeId, u64, State> {
        Order {
            exchange: ExchangeId::Simulated,
            instrument: 1,
            strategy: StrategyId::unknown(),
            cid,
            side: Side::Buy,
            state,
        }
    }

    fn order_snapshot_exchange_cancelled(
        cid: ClientOrderId,
    ) -> Snapshot<Order<ExchangeId, u64, ExchangeOrderState>> {
        Snapshot(Order {
            exchange: ExchangeId::Simulated,
            instrument: 1,
            strategy: StrategyId::unknown(),
            cid,
            side: Side::Buy,
            state: ExchangeOrderState::Cancelled(Cancelled {
                id: OrderId(SmolStr::default()),
                time_exchange: Default::default(),
            }),
        })
    }

    fn order_snapshot_exchange_fully_filled(
        cid: ClientOrderId,
    ) -> Snapshot<Order<ExchangeId, u64, ExchangeOrderState>> {
        Snapshot(Order {
            exchange: ExchangeId::Simulated,
            instrument: 1,
            strategy: StrategyId::unknown(),
            cid,
            side: Side::Buy,
            state: ExchangeOrderState::FullyFilled,
        })
    }

    fn order_snapshot_exchange_expired(
        cid: ClientOrderId,
    ) -> Snapshot<Order<ExchangeId, u64, ExchangeOrderState>> {
        Snapshot(Order {
            exchange: ExchangeId::Simulated,
            instrument: 1,
            strategy: StrategyId::unknown(),
            cid,
            side: Side::Buy,
            state: ExchangeOrderState::Expired,
        })
    }

    fn order_snapshot_exchange_rejected(
        cid: ClientOrderId,
    ) -> Snapshot<Order<ExchangeId, u64, ExchangeOrderState>> {
        Snapshot(Order {
            exchange: ExchangeId::Simulated,
            instrument: 1,
            strategy: StrategyId::unknown(),
            cid,
            side: Side::Buy,
            state: ExchangeOrderState::Rejected(None),
        })
    }

    fn order_snapshot_exchange_open(
        cid: ClientOrderId,
        time_exchange: DateTime<Utc>,
    ) -> Snapshot<Order<ExchangeId, u64, ExchangeOrderState>> {
        Snapshot(Order {
            exchange: ExchangeId::Simulated,
            instrument: 1,
            strategy: StrategyId::unknown(),
            cid,
            side: Side::Buy,
            state: ExchangeOrderState::Open(open(time_exchange)),
        })
    }

    fn open(time_exchange: DateTime<Utc>) -> Open {
        Open {
            id: OrderId(SmolStr::default()),
            time_exchange,
            price: dec!(1),
            quantity: dec!(1),
            filled_quantity: Default::default(),
        }
    }

    fn cancelled() -> Cancelled {
        Cancelled {
            id: OrderId(SmolStr::default()),
            time_exchange: Default::default(),
        }
    }

    fn request_cancels(
        orders: impl IntoIterator<Item = Order<ExchangeId, u64, RequestCancel>>,
    ) -> FnvHashMap<ClientOrderId, Order<ExchangeId, u64, InternalOrderState>> {
        orders
            .into_iter()
            .map(|order| (order.cid.clone(), Order::from(&order)))
            .collect()
    }

    fn request_cancel(cid: ClientOrderId) -> Order<ExchangeId, u64, RequestCancel> {
        Order {
            exchange: ExchangeId::Simulated,
            instrument: 1,
            strategy: StrategyId::unknown(),
            cid,
            side: Side::Buy,
            state: RequestCancel { id: None },
        }
    }

    fn request_opens(
        orders: impl IntoIterator<Item = Order<ExchangeId, u64, RequestOpen>>,
    ) -> FnvHashMap<ClientOrderId, Order<ExchangeId, u64, InternalOrderState>> {
        orders
            .into_iter()
            .map(|order| (order.cid.clone(), Order::from(&order)))
            .collect()
    }

    fn request_open(cid: ClientOrderId) -> Order<ExchangeId, u64, RequestOpen> {
        Order {
            exchange: ExchangeId::Simulated,
            instrument: 1,
            strategy: StrategyId::unknown(),
            cid,
            side: Side::Buy,
            state: RequestOpen {
                kind: OrderKind::Limit,
                time_in_force: TimeInForce::GoodUntilEndOfDay,
                price: dec!(0.0),
                quantity: dec!(0.0),
            },
        }
    }

    #[test]
    fn test_update_from_open() {
        struct TestCase {
            state: Orders<ExchangeId, u64>,
            input: Order<ExchangeId, u64, Result<Open, ClientError<u64, u64>>>,
            expected: Orders<ExchangeId, u64>,
        }

        let cid = ClientOrderId::default();

        let cases = vec![
            // TC0: cid existing OpenInFlight, response Ok(Open)
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::OpenInFlight(OpenInFlight),
                )]),
                input: order(cid.clone(), Ok(open(DateTime::default()))),
                expected: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::default())),
                )]),
            },
            // TC1: cid existing Open, response Ok(Open) with more recent timestamp
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::MIN_UTC)),
                )]),
                input: order(cid.clone(), Ok(open(DateTime::<Utc>::MAX_UTC))),
                expected: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::MAX_UTC)),
                )]),
            },
            // TC2: cid existing Open, response Ok(Open) with less recent timestamp
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::MAX_UTC)),
                )]),
                input: order(cid.clone(), Ok(open(DateTime::<Utc>::MIN_UTC))),
                expected: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::MAX_UTC)),
                )]),
            },
            // TC3: cid existing CancelInFlight, response Ok(Open), so ignore response
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
                input: order(cid.clone(), Ok(open(DateTime::default()))),
                expected: orders([order(
                    cid.clone(),
                    InternalOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
            },
            // TC4: cid untracked, response Ok(Open), so add tracking starting as Open
            TestCase {
                state: Orders::default(),
                input: order(cid.clone(), Ok(open(DateTime::default()))),
                expected: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::default())),
                )]),
            },
            // TC5: cid tracked as OpenInFlight, response Err(Timeout), so remove
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::OpenInFlight(OpenInFlight),
                )]),
                input: order(
                    cid.clone(),
                    Err(ClientError::Connectivity(ConnectivityError::Timeout)),
                ),
                expected: Orders::default(),
            },
            // TC6: cid tracked as OpenInFlight, response Err(AlreadyFilled), so remove
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::OpenInFlight(OpenInFlight),
                )]),
                input: order(
                    cid.clone(),
                    Err(ClientError::Api(ApiError::OrderAlreadyFullyFilled)),
                ),
                expected: Orders::default(),
            },
            // TC7: cid tracked as Open, response Err(AlreadyCancelled), so remove
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::default())),
                )]),
                input: order(
                    cid.clone(),
                    Err(ClientError::Api(ApiError::OrderAlreadyCancelled)),
                ),
                expected: Orders::default(),
            },
            // TC8: cid tracked as Open, response Err(AlreadyFilled), so remove
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::default())),
                )]),
                input: order(
                    cid.clone(),
                    Err(ClientError::Api(ApiError::OrderAlreadyFullyFilled)),
                ),
                expected: Orders::default(),
            },
            // TC9: cid tracked as Open, response Err indicating not already completed, so ignore
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::default())),
                )]),
                input: order(cid.clone(), Err(ClientError::Api(ApiError::RateLimit))),
                expected: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::default())),
                )]),
            },
            // TC10: cid tracked as CancelInFlight, response Err(AlreadyCancelled), so remove
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
                input: order(
                    cid.clone(),
                    Err(ClientError::Api(ApiError::OrderAlreadyCancelled)),
                ),
                expected: Orders::default(),
            },
            // TC11: cid tracked as CancelInFlight, response Err(AlreadyFilled), so remove
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
                input: order(
                    cid.clone(),
                    Err(ClientError::Api(ApiError::OrderAlreadyFullyFilled)),
                ),
                expected: Orders::default(),
            },
            // TC12: cid tracked as CancelInFlight, response Err indicating not already completed, so ignore
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
                input: order(
                    cid.clone(),
                    Err(ClientError::Connectivity(ConnectivityError::Timeout)),
                ),
                expected: orders([order(
                    cid.clone(),
                    InternalOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
            },
            // TC13: cid untracked, response Err, so ignore
            TestCase {
                state: Orders::default(),
                input: order(
                    cid.clone(),
                    Err(ClientError::Connectivity(ConnectivityError::Timeout)),
                ),
                expected: Orders::default(),
            },
        ];

        for (index, mut test) in cases.into_iter().enumerate() {
            test.state.update_from_open(&test.input);
            assert_eq!(test.state, test.expected, "TC{index} failed")
        }
    }

    #[test]
    fn test_update_from_cancel() {
        struct TestCase {
            state: Orders<ExchangeId, u64>,
            input: Order<ExchangeId, u64, Result<Cancelled, ClientError<u64, u64>>>,
            expected: Orders<ExchangeId, u64>,
        }

        let cid = ClientOrderId::default();

        let cases = vec![
            // TC0: cid tracked as OpenInFlight, response Ok(Cancelled), so remove
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::OpenInFlight(OpenInFlight),
                )]),
                input: order(cid.clone(), Ok(cancelled())),
                expected: Orders::default(),
            },
            // TC1: cid tracked as Open, response Ok(Cancelled), so remove
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::default())),
                )]),
                input: order(cid.clone(), Ok(cancelled())),
                expected: Orders::default(),
            },
            // TC2: cid tracked as CancelInFlight, response Ok(Cancelled), so remove
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
                input: order(cid.clone(), Ok(cancelled())),
                expected: Orders::default(),
            },
            // TC3: cid untracked, response Ok(Cancelled), so ignore
            TestCase {
                state: Orders::default(),
                input: order(cid.clone(), Ok(cancelled())),
                expected: Orders::default(),
            },
            // TC4: cid tracked as OpenInFlight, response Err(_), so remove
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::OpenInFlight(OpenInFlight),
                )]),
                input: order(cid.clone(), Err(ClientError::Api(ApiError::RateLimit))),
                expected: Orders::default(),
            },
            // TC5: cid tracked as Open, response Err(AlreadyCancelled), so remove
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::default())),
                )]),
                input: order(
                    cid.clone(),
                    Err(ClientError::Api(ApiError::OrderAlreadyCancelled)),
                ),
                expected: Orders::default(),
            },
            // TC6: cid tracked as Open, response Err(AlreadyFilled), so remove
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::default())),
                )]),
                input: order(
                    cid.clone(),
                    Err(ClientError::Api(ApiError::OrderAlreadyFullyFilled)),
                ),
                expected: Orders::default(),
            },
            // TC7: cid tracked as CancelInFlight, response Err(_), so remove
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
                input: order(cid.clone(), Err(ClientError::Api(ApiError::RateLimit))),
                expected: Orders::default(),
            },
            // TC8: cid untracked, response Err(_), so ignore
            TestCase {
                state: Orders::default(),
                input: order(
                    cid.clone(),
                    Err(ClientError::Connectivity(ConnectivityError::Timeout)),
                ),
                expected: Orders::default(),
            },
        ];

        for (index, mut test) in cases.into_iter().enumerate() {
            test.state.update_from_cancel(&test.input);
            assert_eq!(test.state, test.expected, "TC{index} failed")
        }
    }

    #[test]
    fn test_update_from_order_snapshot() {
        struct TestCase {
            state: Orders<ExchangeId, u64>,
            input: Snapshot<Order<ExchangeId, u64, ExchangeOrderState>>,
            expected: Orders<ExchangeId, u64>,
        }

        let cid = ClientOrderId::default();

        let cases = vec![
            // TC0: Cancel tracked Order<OpenInFlight> after receiving Snapshot<Order<Cancelled>>
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::OpenInFlight(OpenInFlight),
                )]),
                input: order_snapshot_exchange_cancelled(cid.clone()),
                expected: Orders::default(),
            },
            // TC1: Cancel tracked Order<Open> after receiving Snapshot<Order<Cancelled>>
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::default())),
                )]),
                input: order_snapshot_exchange_cancelled(cid.clone()),
                expected: Orders::default(),
            },
            // TC2: Cancel tracked Order<CancelInFlight> after receiving Snapshot<Order<Cancelled>>
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
                input: order_snapshot_exchange_cancelled(cid.clone()),
                expected: Orders::default(),
            },
            // TC3: remove tracked Order<OpenInFlight> after receiving Snapshot<Order<FullyFilled>>
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::OpenInFlight(OpenInFlight),
                )]),
                input: order_snapshot_exchange_fully_filled(cid.clone()),
                expected: Orders::default(),
            },
            // TC4: remove tracked Order<Open> after receiving Snapshot<Order<FullyFilled>>
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::default())),
                )]),
                input: order_snapshot_exchange_fully_filled(cid.clone()),
                expected: Orders::default(),
            },
            // TC5: remove tracked Order<CancelInFlight> after receiving Snapshot<Order<FullyFilled>>
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
                input: order_snapshot_exchange_fully_filled(cid.clone()),
                expected: Orders::default(),
            },
            // TC6: remove tracked Order<OpenInFlight> after receiving Snapshot<Order<Expired>>
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::OpenInFlight(OpenInFlight),
                )]),
                input: order_snapshot_exchange_expired(cid.clone()),
                expected: Orders::default(),
            },
            // TC7: remove tracked Order<Open> after receiving Snapshot<Order<Expired>>
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::default())),
                )]),
                input: order_snapshot_exchange_expired(cid.clone()),
                expected: Orders::default(),
            },
            // TC8: remove tracked Order<CancelInFlight> after receiving Snapshot<Order<Expired>>
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
                input: order_snapshot_exchange_expired(cid.clone()),
                expected: Orders::default(),
            },
            // TC9: Open tracked Order<OpenInFlight> after receiving Snapshot<Order<Open>>
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::OpenInFlight(OpenInFlight),
                )]),
                input: order_snapshot_exchange_open(cid.clone(), DateTime::<Utc>::default()),
                expected: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::default())),
                )]),
            },
            // TC10: Update tracked Order<Open> after receiving more recent Snapshot<Order<Open>>
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::MIN_UTC)),
                )]),
                input: order_snapshot_exchange_open(cid.clone(), DateTime::<Utc>::MAX_UTC),
                expected: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::MAX_UTC)),
                )]),
            },
            // TC11: Ignore stale Snapshot<Order<Open>> when tracked Order<Open> is more recent
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::MAX_UTC)),
                )]),
                input: order_snapshot_exchange_open(cid.clone(), DateTime::<Utc>::MIN_UTC),
                expected: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::MAX_UTC)),
                )]),
            },
            // TC12: Ignore stale Snapshot<Order<Open>> when internal state is Order<CancelInFlight>
            TestCase {
                state: orders([order(
                    cid.clone(),
                    InternalOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
                input: order_snapshot_exchange_open(cid.clone(), DateTime::<Utc>::default()),
                expected: orders([order(
                    cid.clone(),
                    InternalOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
            },
            // TC13: Ignore Snapshot<Order<Cancelled>> for untracked Order
            TestCase {
                state: Orders::default(),
                input: order_snapshot_exchange_cancelled(cid.clone()),
                expected: Orders::default(),
            },
            // TC14: Ignore Snapshot<Order<FullyFilled>> for untracked Order
            TestCase {
                state: Orders::default(),
                input: order_snapshot_exchange_fully_filled(cid.clone()),
                expected: Orders::default(),
            },
            // TC15: Ignore Snapshot<Order<Expired>> for untracked Order
            TestCase {
                state: Orders::default(),
                input: order_snapshot_exchange_expired(cid.clone()),
                expected: Orders::default(),
            },
            // TC15: Ignore Snapshot<Order<Rejected>> for untracked Order
            TestCase {
                state: Orders::default(),
                input: order_snapshot_exchange_rejected(cid.clone()),
                expected: Orders::default(),
            },
            // TC16: Insert untracked Snapshot<Order<Open>>
            TestCase {
                state: Orders::default(),
                input: order_snapshot_exchange_open(cid.clone(), DateTime::<Utc>::default()),
                expected: orders([order(
                    cid.clone(),
                    InternalOrderState::Open(open(DateTime::<Utc>::default())),
                )]),
            },
        ];

        for (index, mut test) in cases.into_iter().enumerate() {
            test.state.update_from_order_snapshot(test.input.as_ref());
            assert_eq!(test.state, test.expected, "TC{index} failed")
        }
    }

    #[test]
    fn test_record_in_flight_cancel() {
        struct TestCase {
            state: Orders<ExchangeId, u64>,
            input: Vec<Order<ExchangeId, u64, RequestCancel>>,
            expected: Orders<ExchangeId, u64>,
        }

        let cid_1 = ClientOrderId::default();
        let cid_2 = ClientOrderId::default();

        let cases = vec![
            TestCase {
                // TC0: Insert untracked InFlight
                state: Orders::default(),
                input: vec![request_cancel(cid_1.clone())],
                expected: Orders(request_cancels([request_cancel(cid_1.clone())])),
            },
            TestCase {
                // TC1: Insert InFlight that is already tracked
                state: Orders(request_cancels([request_cancel(cid_1.clone())])),
                input: vec![request_cancel(cid_1.clone())],
                expected: Orders(request_cancels([request_cancel(cid_1.clone())])),
            },
            TestCase {
                // TC2: Insert one untracked InFlight, and one already tracked
                state: Orders(request_cancels([request_cancel(cid_1.clone())])),
                input: vec![request_cancel(cid_1.clone()), request_cancel(cid_2.clone())],
                expected: Orders(request_cancels([
                    request_cancel(cid_1),
                    request_cancel(cid_2),
                ])),
            },
        ];

        for (index, mut test) in cases.into_iter().enumerate() {
            for in_flight in test.input {
                test.state.record_in_flight_cancel(&in_flight);
            }
            assert_eq!(test.state, test.expected, "TC{index} failed")
        }
    }

    #[test]
    fn test_record_in_flight_open() {
        struct TestCase {
            state: Orders<ExchangeId, u64>,
            input: Vec<Order<ExchangeId, u64, RequestOpen>>,
            expected: Orders<ExchangeId, u64>,
        }

        let cid_1 = ClientOrderId::default();
        let cid_2 = ClientOrderId::default();

        let cases = vec![
            TestCase {
                // TC0: Insert unseen InFlight
                state: Orders::default(),
                input: vec![request_open(cid_1.clone())],
                expected: Orders(request_opens([request_open(cid_1.clone())])),
            },
            TestCase {
                // TC1: Insert InFlight that is already tracked
                state: Orders(request_opens([request_open(cid_1.clone())])),
                input: vec![request_open(cid_1.clone())],
                expected: Orders(request_opens([request_open(cid_1.clone())])),
            },
            TestCase {
                // TC2: Insert one untracked InFlight, and one already tracked
                state: Orders(request_opens([request_open(cid_1.clone())])),
                input: vec![request_open(cid_1.clone()), request_open(cid_2.clone())],
                expected: Orders(request_opens([request_open(cid_1), request_open(cid_2)])),
            },
        ];

        for (index, mut test) in cases.into_iter().enumerate() {
            for in_flight in test.input {
                test.state.record_in_flight_open(&in_flight);
            }
            assert_eq!(test.state, test.expected, "TC{index} failed")
        }
    }
}
