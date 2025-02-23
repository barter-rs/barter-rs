use crate::engine::state::order::{
    in_flight_recorder::InFlightRequestRecorder, manager::OrderManager,
};
use barter_execution::order::{
    Order,
    id::ClientOrderId,
    request::{OrderRequestCancel, OrderRequestOpen, OrderResponseCancel},
    state::{ActiveOrderState, CancelInFlight, OrderState},
};
use barter_instrument::{exchange::ExchangeIndex, instrument::InstrumentIndex};
use barter_integration::snapshot::Snapshot;
use derive_more::Constructor;
use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};
use std::{collections::hash_map::Entry, fmt::Debug};
use tracing::{debug, error, warn};

pub mod in_flight_recorder;
pub mod manager;

/// Synchronous order manager that tracks the lifecycle of active exchange orders.
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
/// Orders tend to progress through the following states:
/// 1. OpenInFlight - Initial order request sent to exchange
/// 2. Open - Order confirmed as open on exchange
/// 3. CancelInFlight - Cancellation request sent to exchange
/// 4. Cancelled/Expired/FullyFilled - Terminal states, once achieved order is no longer tracked.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct Orders<ExchangeKey = ExchangeIndex, InstrumentKey = InstrumentIndex>(
    pub FnvHashMap<ClientOrderId, Order<ExchangeKey, InstrumentKey, ActiveOrderState>>,
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
    ) -> impl Iterator<Item = &'a Order<ExchangeKey, InstrumentKey, ActiveOrderState>>
    where
        ExchangeKey: 'a,
        InstrumentKey: 'a,
    {
        self.0.values()
    }

    fn update_from_order_snapshot<AssetKey>(
        &mut self,
        snapshot: Snapshot<&Order<ExchangeKey, InstrumentKey, OrderState<AssetKey, InstrumentKey>>>,
    ) where
        AssetKey: Debug + Clone,
    {
        let Snapshot(snapshot) = snapshot;

        let (mut current_entry, update) = match (
            self.0.entry(snapshot.key.cid.clone()),
            snapshot.to_active(),
        ) {
            // Order untracked, input Snapshot is InactiveOrderState (ie/ finished), so ignore
            (Entry::Vacant(_), None) => {
                warn!(
                    exchange = ?snapshot.key.exchange,
                    instrument = ?snapshot.key.instrument,
                    strategy = %snapshot.key.strategy,
                    cid = %snapshot.key.cid,
                    update = ?snapshot,
                    "OrderManager received inactive order snapshot for untracked order - ignoring"
                );
                return;
            }

            // Order untracked, input Snapshot is ActiveOrderState, so insert
            (Entry::Vacant(entry), Some(update)) => {
                match &update.state {
                    ActiveOrderState::Open(open)
                        if open.quantity_remaining(update.quantity).is_zero() =>
                    {
                        debug!(
                            exchange = ?snapshot.key.exchange,
                            instrument = ?snapshot.key.instrument,
                            strategy = %snapshot.key.strategy,
                            cid = %snapshot.key.cid,
                            update = ?snapshot,
                            "OrderManager ignoring new Open order which is actually FulledFilled"
                        );
                    }
                    _active_order => {
                        debug!(
                            exchange = ?snapshot.key.exchange,
                            instrument = ?snapshot.key.instrument,
                            strategy = %snapshot.key.strategy,
                            cid = %snapshot.key.cid,
                            update = ?snapshot,
                            "OrderManager tracking new order"
                        );
                        entry.insert(update);
                    }
                }
                return;
            }

            // Order tracked, input Snapshot is InactiveOrderState (ie/ finished), so remove
            (Entry::Occupied(entry), None) => {
                debug!(
                    exchange = ?snapshot.key.exchange,
                    instrument = ?snapshot.key.instrument,
                    strategy = %snapshot.key.strategy,
                    cid = %snapshot.key.cid,
                    update = ?snapshot,
                    "OrderManager received inactive order snapshot for tracked order - removing"
                );
                entry.remove();
                return;
            }

            // Order tracked, input Snapshot is ActiveOrderState, so forward for further processing
            (Entry::Occupied(entry), Some(update)) => (entry, update),
        };

        match (&current_entry.get().state, update.state) {
            (ActiveOrderState::OpenInFlight(_), ActiveOrderState::OpenInFlight(_)) => {
                warn!(
                    exchange = ?snapshot.key.exchange,
                    instrument = ?snapshot.key.instrument,
                    strategy = %snapshot.key.strategy,
                    cid = %snapshot.key.cid,
                    update = ?snapshot,
                    "OrderManager received a duplicate OpenInFlight recording - ignoring"
                );
            }
            (ActiveOrderState::OpenInFlight(_), ActiveOrderState::Open(open)) => {
                debug!(
                    exchange = ?snapshot.key.exchange,
                    instrument = ?snapshot.key.instrument,
                    strategy = %snapshot.key.strategy,
                    cid = %snapshot.key.cid,
                    update = ?snapshot,
                    "OrderManager transitioned an OpenInFlight order to Open"
                );
                if open.quantity_remaining(update.quantity).is_zero() {
                    current_entry.remove();
                } else {
                    current_entry.get_mut().state = ActiveOrderState::Open(open);
                }
            }
            (ActiveOrderState::OpenInFlight(_), ActiveOrderState::CancelInFlight(update)) => {
                debug!(
                    exchange = ?snapshot.key.exchange,
                    instrument = ?snapshot.key.instrument,
                    strategy = %snapshot.key.strategy,
                    cid = %snapshot.key.cid,
                    update = ?snapshot,
                    "OrderManager transitioned an OpenInFlight order to CancelInFlight"
                );
                current_entry.get_mut().state = ActiveOrderState::CancelInFlight(update);
            }
            (ActiveOrderState::Open(_), ActiveOrderState::OpenInFlight(_)) => {
                warn!(
                    exchange = ?snapshot.key.exchange,
                    instrument = ?snapshot.key.instrument,
                    strategy = %snapshot.key.strategy,
                    cid = %snapshot.key.cid,
                    update = ?snapshot,
                    "OrderManager received an OpenInFlight recording for an Open order - ignoring"
                );
            }
            (ActiveOrderState::Open(current), ActiveOrderState::Open(update)) => {
                if current.time_exchange <= update.time_exchange {
                    debug!(
                        exchange = ?snapshot.key.exchange,
                        instrument = ?snapshot.key.instrument,
                        strategy = %snapshot.key.strategy,
                        cid = %snapshot.key.cid,
                        update = ?snapshot,
                        "OrderManager updating an Open order from a more recent snapshot"
                    );
                    current_entry.get_mut().state = ActiveOrderState::Open(update);
                } else {
                    debug!(
                        exchange = ?snapshot.key.exchange,
                        instrument = ?snapshot.key.instrument,
                        strategy = %snapshot.key.strategy,
                        cid = %snapshot.key.cid,
                        update = ?snapshot,
                        "OrderManager received an out of sequence Open order snapshot - ignoring"
                    );
                }
            }
            (ActiveOrderState::Open(current), ActiveOrderState::CancelInFlight(mut update)) => {
                debug!(
                    exchange = ?snapshot.key.exchange,
                    instrument = ?snapshot.key.instrument,
                    strategy = %snapshot.key.strategy,
                    cid = %snapshot.key.cid,
                    update = ?snapshot,
                    "OrderManager transitioned an Open order to CancelInFlight"
                );

                // Ensure next CancelInFlight.Open is populated and the most recent
                let latest_open = update
                    .order
                    .take()
                    .filter(|update| current.time_exchange <= update.time_exchange)
                    .unwrap_or_else(|| current.clone());

                current_entry.get_mut().state = ActiveOrderState::CancelInFlight(CancelInFlight {
                    order: Some(latest_open),
                })
            }
            (ActiveOrderState::CancelInFlight(_), ActiveOrderState::OpenInFlight(_)) => {
                error!(
                    exchange = ?snapshot.key.exchange,
                    instrument = ?snapshot.key.instrument,
                    strategy = %snapshot.key.strategy,
                    cid = %snapshot.key.cid,
                    update = ?snapshot,
                    "OrderManager received an OpenInFlight recording for a CancelInFlight - ignoring"
                );
            }
            (ActiveOrderState::CancelInFlight(current), ActiveOrderState::Open(update)) => {
                debug!(
                    exchange = ?snapshot.key.exchange,
                    instrument = ?snapshot.key.instrument,
                    strategy = %snapshot.key.strategy,
                    cid = %snapshot.key.cid,
                    update = ?snapshot,
                    "OrderManager received an Open order snapshot for a CancelInFlight - updating CancelInFlight.Open"
                );

                // Check if the update Open is more recent
                let update_open_is_latest = current
                    .order
                    .as_ref()
                    .is_none_or(|current| current.time_exchange <= update.time_exchange);

                if update_open_is_latest {
                    current_entry.get_mut().state =
                        ActiveOrderState::CancelInFlight(CancelInFlight {
                            order: Some(update),
                        });
                }
            }
            (ActiveOrderState::CancelInFlight(_), ActiveOrderState::CancelInFlight(_)) => {
                warn!(
                    exchange = ?snapshot.key.exchange,
                    instrument = ?snapshot.key.instrument,
                    strategy = %snapshot.key.strategy,
                    cid = %snapshot.key.cid,
                    update = ?snapshot,
                    "OrderManager received a duplicate CancelInFlight recording - ignoring"
                );
            }
        }
    }

    fn update_from_cancel_response<AssetKey>(
        &mut self,
        response: &OrderResponseCancel<ExchangeKey, AssetKey, InstrumentKey>,
    ) where
        AssetKey: Debug + Clone,
    {
        let Entry::Occupied(mut order) = self.0.entry(response.key.cid.clone()) else {
            warn!(
                exchange = ?response.key.exchange,
                instrument = ?response.key.instrument,
                strategy = %response.key.strategy,
                cid = %response.key.cid,
                update = ?response,
                "OrderManager received an OrderResponseCancel for untracked order - ignoring"
            );
            return;
        };

        match (&order.get().state, &response.state) {
            (ActiveOrderState::OpenInFlight(_) | ActiveOrderState::Open(_), Ok(_)) => {
                warn!(
                    exchange = ?response.key.exchange,
                    instrument = ?response.key.instrument,
                    strategy = %response.key.strategy,
                    cid = %response.key.cid,
                    update = ?response,
                    "OrderManager received Ok(Cancelled) for tracked order not CancelInFlight - removing"
                );
                order.remove();
            }
            (ActiveOrderState::CancelInFlight(_), Ok(_)) => {
                debug!(
                    exchange = ?response.key.exchange,
                    instrument = ?response.key.instrument,
                    strategy = %response.key.strategy,
                    cid = %response.key.cid,
                    update = ?response,
                    "OrderManager received Ok(Cancelled) for tracked order CancelInFlight - removing"
                );
                order.remove();
            }
            (ActiveOrderState::OpenInFlight(_) | ActiveOrderState::Open(_), Err(error)) => {
                warn!(
                    exchange = ?response.key.exchange,
                    instrument = ?response.key.instrument,
                    strategy = %response.key.strategy,
                    cid = %response.key.cid,
                    update = ?response,
                    ?error,
                    "OrderManager received Err(Cancelled) for tracked order not CancelInFlight - ignoring"
                );
            }
            (ActiveOrderState::CancelInFlight(in_flight_cancel), Err(error)) => {
                // Expected, keep move to Open
                if let Some(open) = &in_flight_cancel.order {
                    debug!(
                        exchange = ?response.key.exchange,
                        instrument = ?response.key.instrument,
                        strategy = %response.key.strategy,
                        cid = %response.key.cid,
                        update = ?response,
                        ?error,
                        "OrderManager received Err(Cancelled) for previously Open order - setting Open"
                    );
                    order.get_mut().state = ActiveOrderState::Open(open.clone())
                } else {
                    debug!(
                        exchange = ?response.key.exchange,
                        instrument = ?response.key.instrument,
                        strategy = %response.key.strategy,
                        cid = %response.key.cid,
                        update = ?response,
                        ?error,
                        "OrderManager received Err(Cancelled) for previously non-Open order - removing"
                    );
                    // Likely previously OpenInFlight, and attempted cancel before Open snapshot
                    // -> it's expected that an Order snapshot is inbound
                    order.remove();
                }
            }
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
        request: &OrderRequestCancel<ExchangeKey, InstrumentKey>,
    ) {
        let Some(order) = self.0.get_mut(&request.key.cid) else {
            error!(
                cid = %request.key.cid,
                event = ?request,
                "OrderManager cannot mark CancelInFlight for untracked Order - ignoring"
            );
            return;
        };

        order.state = ActiveOrderState::CancelInFlight(CancelInFlight {
            order: order.state.open_meta().cloned(),
        });
    }

    fn record_in_flight_open(&mut self, request: &OrderRequestOpen<ExchangeKey, InstrumentKey>) {
        if let Some(duplicate_cid_order) =
            self.0.insert(request.key.cid.clone(), Order::from(request))
        {
            error!(
                cid = %duplicate_cid_order.key.cid,
                event = ?duplicate_cid_order,
                "OrderManager upserted Order OpenInFlight with duplicate ClientOrderId"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{engine::state::order::Orders, test_utils::time_plus_secs};
    use barter_execution::{
        error::{ConnectivityError, OrderError},
        order::{
            Order, OrderKey, OrderKind, TimeInForce,
            id::{ClientOrderId, OrderId, StrategyId},
            request::{RequestCancel, RequestOpen},
            state::{ActiveOrderState, CancelInFlight, Cancelled, Open, OpenInFlight},
        },
    };
    use barter_instrument::{Side, exchange::ExchangeId};
    use chrono::{DateTime, Utc};
    use rust_decimal_macros::dec;
    use smol_str::SmolStr;

    fn orders(
        orders: impl IntoIterator<Item = Order<ExchangeId, u64, ActiveOrderState>>,
    ) -> Orders<ExchangeId, u64> {
        Orders(
            orders
                .into_iter()
                .map(|order| (order.key.cid.clone(), order))
                .collect(),
        )
    }

    fn order<State>(cid: ClientOrderId, state: State) -> Order<ExchangeId, u64, State> {
        Order {
            key: OrderKey {
                exchange: ExchangeId::Simulated,
                instrument: 1,
                strategy: StrategyId::unknown(),
                cid,
            },
            side: Side::Buy,
            price: dec!(1),
            quantity: dec!(1),
            kind: OrderKind::Limit,
            time_in_force: TimeInForce::GoodUntilCancelled { post_only: false },
            state,
        }
    }

    fn order_cancel_in_flight(cid: ClientOrderId) -> Order<ExchangeId, u64, ActiveOrderState> {
        order(
            cid,
            ActiveOrderState::CancelInFlight(CancelInFlight { order: None }),
        )
    }

    fn order_snapshot_cancelled(
        cid: ClientOrderId,
    ) -> Snapshot<Order<ExchangeId, u64, OrderState<u64, u64>>> {
        Snapshot(Order {
            key: OrderKey {
                exchange: ExchangeId::Simulated,
                instrument: 1,
                strategy: StrategyId::unknown(),
                cid,
            },
            side: Side::Buy,
            price: Default::default(),
            quantity: Default::default(),
            kind: OrderKind::Market,
            time_in_force: TimeInForce::GoodUntilEndOfDay,
            state: OrderState::inactive(Cancelled {
                id: OrderId(SmolStr::default()),
                time_exchange: Default::default(),
            }),
        })
    }

    fn order_snapshot_fully_filled(
        cid: ClientOrderId,
    ) -> Snapshot<Order<ExchangeId, u64, OrderState<u64, u64>>> {
        Snapshot(Order {
            key: OrderKey {
                exchange: ExchangeId::Simulated,
                instrument: 1,
                strategy: StrategyId::unknown(),
                cid,
            },
            side: Side::Buy,
            price: Default::default(),
            quantity: Default::default(),
            kind: OrderKind::Market,
            time_in_force: TimeInForce::GoodUntilEndOfDay,
            state: OrderState::fully_filled(),
        })
    }

    fn order_snapshot_failed(
        cid: ClientOrderId,
    ) -> Snapshot<Order<ExchangeId, u64, OrderState<u64, u64>>> {
        Snapshot(Order {
            key: OrderKey {
                exchange: ExchangeId::Simulated,
                instrument: 1,
                strategy: StrategyId::unknown(),
                cid,
            },
            side: Side::Buy,
            price: Default::default(),
            quantity: Default::default(),
            kind: OrderKind::Market,
            time_in_force: TimeInForce::GoodUntilEndOfDay,
            state: OrderState::inactive(OrderError::Connectivity(ConnectivityError::Timeout)),
        })
    }

    fn order_snapshot_expired(
        cid: ClientOrderId,
    ) -> Snapshot<Order<ExchangeId, u64, OrderState<u64, u64>>> {
        Snapshot(Order {
            key: OrderKey {
                exchange: ExchangeId::Simulated,
                instrument: 1,
                strategy: StrategyId::unknown(),
                cid,
            },
            side: Side::Buy,
            price: Default::default(),
            quantity: Default::default(),
            kind: OrderKind::Market,
            time_in_force: TimeInForce::GoodUntilEndOfDay,
            state: OrderState::expired(),
        })
    }

    fn order_snapshot_open(
        cid: ClientOrderId,
        time_exchange: DateTime<Utc>,
    ) -> Snapshot<Order<ExchangeId, u64, OrderState<u64, u64>>> {
        Snapshot(Order {
            key: OrderKey {
                exchange: ExchangeId::Simulated,
                instrument: 1,
                strategy: StrategyId::unknown(),
                cid,
            },
            side: Side::Buy,
            price: dec!(1),
            quantity: dec!(1),
            kind: OrderKind::Limit,
            time_in_force: TimeInForce::GoodUntilCancelled { post_only: false },
            state: OrderState::active(open(time_exchange)),
        })
    }

    fn open(time_exchange: DateTime<Utc>) -> Open {
        Open {
            id: OrderId(SmolStr::default()),
            time_exchange,
            filled_quantity: Default::default(),
        }
    }

    fn request_cancel(cid: ClientOrderId) -> OrderRequestCancel<ExchangeId, u64> {
        OrderRequestCancel {
            key: OrderKey {
                exchange: ExchangeId::Simulated,
                instrument: 1,
                strategy: StrategyId::unknown(),
                cid,
            },
            state: RequestCancel::default(),
        }
    }

    fn request_opens(
        orders: impl IntoIterator<Item = OrderRequestOpen<ExchangeId, u64>>,
    ) -> FnvHashMap<ClientOrderId, Order<ExchangeId, u64, ActiveOrderState>> {
        orders
            .into_iter()
            .map(|order| (order.key.cid.clone(), Order::from(&order)))
            .collect()
    }

    fn request_open(cid: ClientOrderId) -> OrderRequestOpen<ExchangeId, u64> {
        OrderRequestOpen {
            key: OrderKey {
                exchange: ExchangeId::Simulated,
                instrument: 1,
                strategy: StrategyId::unknown(),
                cid,
            },
            state: RequestOpen {
                side: Side::Buy,
                price: dec!(1),
                quantity: dec!(1),
                kind: OrderKind::Limit,
                time_in_force: TimeInForce::GoodUntilEndOfDay,
            },
        }
    }

    fn response_cancel_ok(cid: ClientOrderId) -> OrderResponseCancel<ExchangeId, u64, u64> {
        OrderResponseCancel {
            key: OrderKey {
                exchange: ExchangeId::Simulated,
                instrument: 1,
                strategy: StrategyId::unknown(),
                cid,
            },
            state: Ok(Cancelled {
                id: OrderId(SmolStr::default()),
                time_exchange: DateTime::<Utc>::MIN_UTC,
            }),
        }
    }

    fn response_cancel_err(cid: ClientOrderId) -> OrderResponseCancel<ExchangeId, u64, u64> {
        OrderResponseCancel {
            key: OrderKey {
                exchange: ExchangeId::Simulated,
                instrument: 1,
                strategy: StrategyId::unknown(),
                cid,
            },
            state: Err(OrderError::Connectivity(ConnectivityError::Timeout)),
        }
    }

    #[test]
    fn test_update_from_order_snapshot() {
        struct TestCase {
            name: &'static str,
            state: Orders<ExchangeId, u64>,
            input: Snapshot<Order<ExchangeId, u64, OrderState<u64, u64>>>,
            expected: Orders<ExchangeId, u64>,
        }

        let time_base = DateTime::<Utc>::MIN_UTC;
        let cid = ClientOrderId::default();

        let cases = vec![
            TestCase {
                name: "untracked, Snapshot is inactive, so ignore",
                state: Orders::default(),
                input: order_snapshot_expired(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "untracked, Snapshot is active, so insert",
                state: Orders::default(),
                input: order_snapshot_open(cid.clone(), time_base),
                expected: orders([order(cid.clone(), ActiveOrderState::from(open(time_base)))]),
            },
            TestCase {
                name: "untracked, Snapshot is active Open but fully filled, so ignore",
                state: Orders::default(),
                input: Snapshot(order(
                    cid.clone(),
                    OrderState::active(Open {
                        id: OrderId(SmolStr::default()),
                        time_exchange: time_base,
                        filled_quantity: dec!(1),
                    }),
                )),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked OpenInFlight, Snapshot is inactive cancelled, so remove",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::OpenInFlight(OpenInFlight),
                )]),
                input: order_snapshot_cancelled(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked OpenInFlight, Snapshot is inactive fully filled, so remove",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::OpenInFlight(OpenInFlight),
                )]),
                input: order_snapshot_fully_filled(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked OpenInFlight, Snapshot is inactive failed, so remove",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::OpenInFlight(OpenInFlight),
                )]),
                input: order_snapshot_failed(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked OpenInFlight, Snapshot is inactive expired, so remove",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::OpenInFlight(OpenInFlight),
                )]),
                input: order_snapshot_expired(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked Open, Snapshot is inactive cancelled, so remove",
                state: orders([order(cid.clone(), ActiveOrderState::Open(open(time_base)))]),
                input: order_snapshot_cancelled(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked Open, Snapshot is inactive fully filled, so remove",
                state: orders([order(cid.clone(), ActiveOrderState::Open(open(time_base)))]),
                input: order_snapshot_fully_filled(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked Open, Snapshot is inactive failed, so remove",
                state: orders([order(cid.clone(), ActiveOrderState::Open(open(time_base)))]),
                input: order_snapshot_failed(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked Open, Snapshot is inactive expired, so remove",
                state: orders([order(cid.clone(), ActiveOrderState::Open(open(time_base)))]),
                input: order_snapshot_expired(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked CancelInFlight, Snapshot is inactive cancelled, so remove",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { order: None }),
                )]),
                input: order_snapshot_cancelled(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked CancelInFlight, Snapshot is inactive fully filled, so remove",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { order: None }),
                )]),
                input: order_snapshot_fully_filled(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked CancelInFlight, Snapshot is inactive failed, so remove",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { order: None }),
                )]),
                input: order_snapshot_failed(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked CancelInFlight, Snapshot is inactive expired, so remove",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { order: None }),
                )]),
                input: order_snapshot_expired(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked OpenInFlight, Snapshot is active OpenInFlight, so ignore duplicate",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::OpenInFlight(OpenInFlight),
                )]),
                input: Snapshot(order(cid.clone(), OrderState::active(OpenInFlight))),
                expected: orders([order(
                    cid.clone(),
                    ActiveOrderState::OpenInFlight(OpenInFlight),
                )]),
            },
            TestCase {
                name: "tracked OpenInFlight, Snapshot is active Open but fully filled, so remove",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::OpenInFlight(OpenInFlight),
                )]),
                input: Snapshot(order(
                    cid.clone(),
                    OrderState::active(Open {
                        id: OrderId(SmolStr::default()),
                        time_exchange: time_base,
                        filled_quantity: dec!(1),
                    }),
                )),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked OpenInFlight, Snapshot is active Open, so update",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::OpenInFlight(OpenInFlight),
                )]),
                input: order_snapshot_open(cid.clone(), time_base),
                expected: orders([order(cid.clone(), ActiveOrderState::Open(open(time_base)))]),
            },
            TestCase {
                name: "tracked OpenInFlight, Snapshot is active CancelInFlight, so update",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::OpenInFlight(OpenInFlight),
                )]),
                input: Snapshot(order(
                    cid.clone(),
                    OrderState::active(CancelInFlight { order: None }),
                )),
                expected: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { order: None }),
                )]),
            },
            TestCase {
                name: "tracked Open, Snapshot is active OpenInFlight, so ignore",
                state: orders([order(cid.clone(), ActiveOrderState::Open(open(time_base)))]),
                input: Snapshot(order(cid.clone(), OrderState::active(OpenInFlight))),
                expected: orders([order(cid.clone(), ActiveOrderState::Open(open(time_base)))]),
            },
            TestCase {
                name: "tracked Open, Snapshot is active Open with newer time, so update",
                state: orders([order(cid.clone(), ActiveOrderState::Open(open(time_base)))]),
                input: Snapshot(order(
                    cid.clone(),
                    OrderState::active(ActiveOrderState::Open(open(time_plus_secs(time_base, 1)))),
                )),
                expected: orders([order(
                    cid.clone(),
                    ActiveOrderState::Open(open(time_plus_secs(time_base, 1))),
                )]),
            },
            TestCase {
                name: "tracked Open, Snapshot is active Open with older time, so ignore",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::Open(open(time_plus_secs(time_base, 1))),
                )]),
                input: Snapshot(order(
                    cid.clone(),
                    OrderState::active(ActiveOrderState::Open(open(time_base))),
                )),
                expected: orders([order(
                    cid.clone(),
                    ActiveOrderState::Open(open(time_plus_secs(time_base, 1))),
                )]),
            },
            TestCase {
                name: "tracked Open, Snapshot is active CancelInFlight w/ newer Open, update accordingly",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::Open(open(time_plus_secs(time_base, 1))),
                )]),
                input: Snapshot(order(
                    cid.clone(),
                    OrderState::active(CancelInFlight {
                        order: Some(open(time_plus_secs(time_base, 2))),
                    }),
                )),
                expected: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight {
                        order: Some(open(time_plus_secs(time_base, 2))),
                    }),
                )]),
            },
            TestCase {
                name: "tracked Open, Snapshot is active CancelInFlight w/ older Open, update accordingly",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::Open(open(time_plus_secs(time_base, 2))),
                )]),
                input: Snapshot(order(
                    cid.clone(),
                    OrderState::active(CancelInFlight {
                        order: Some(open(time_plus_secs(time_base, 1))),
                    }),
                )),
                expected: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight {
                        order: Some(open(time_plus_secs(time_base, 2))),
                    }),
                )]),
            },
            TestCase {
                name: "tracked Open, Snapshot is active CancelInFlight w/ None Open, update accordingly",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::Open(open(time_plus_secs(time_base, 1))),
                )]),
                input: Snapshot(order(
                    cid.clone(),
                    OrderState::active(CancelInFlight { order: None }),
                )),
                expected: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight {
                        order: Some(open(time_plus_secs(time_base, 1))),
                    }),
                )]),
            },
            TestCase {
                name: "tracked CancelInFlight, Snapshot is active OpenInFlight, so ignore",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { order: None }),
                )]),
                input: Snapshot(order(cid.clone(), OrderState::active(OpenInFlight))),
                expected: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { order: None }),
                )]),
            },
            TestCase {
                name: "tracked CancelInFlight w/ None Open, Snapshot is active Open, update accordingly",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { order: None }),
                )]),
                input: order_snapshot_open(cid.clone(), time_base),
                expected: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight {
                        order: Some(open(time_plus_secs(time_base, 0))),
                    }),
                )]),
            },
            TestCase {
                name: "tracked CancelInFlight, Snapshot is active Open w/ older time, so ignore",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight {
                        order: Some(open(time_plus_secs(time_base, 2))),
                    }),
                )]),
                input: order_snapshot_open(cid.clone(), time_plus_secs(time_base, 1)),
                expected: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight {
                        order: Some(open(time_plus_secs(time_base, 2))),
                    }),
                )]),
            },
            TestCase {
                name: "tracked CancelInFlight, Snapshot is active Open w/ newer time, so update accordingly",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight {
                        order: Some(open(time_plus_secs(time_base, 1))),
                    }),
                )]),
                input: order_snapshot_open(cid.clone(), time_plus_secs(time_base, 2)),
                expected: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight {
                        order: Some(open(time_plus_secs(time_base, 2))),
                    }),
                )]),
            },
            TestCase {
                name: "tracked CancelInFlight, Snapshot is active CancelInFlight, so ignore duplicate",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { order: None }),
                )]),
                input: Snapshot(order(
                    cid.clone(),
                    OrderState::active(CancelInFlight { order: None }),
                )),
                expected: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { order: None }),
                )]),
            },
        ];

        for mut test in cases.into_iter() {
            test.state.update_from_order_snapshot(test.input.as_ref());
            assert_eq!(test.state, test.expected, "TC failed: {}", test.name)
        }
    }

    #[test]
    fn test_update_from_cancel_response() {
        struct TestCase {
            name: &'static str,
            state: Orders<ExchangeId, u64>,
            input: OrderResponseCancel<ExchangeId, u64, u64>,
            expected: Orders<ExchangeId, u64>,
        }

        let cid = ClientOrderId::default();
        let time_base = DateTime::<Utc>::MIN_UTC;

        let cases = vec![
            TestCase {
                name: "untracked, so ignore",
                state: Orders::default(),
                input: response_cancel_ok(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked OpenInFlight, response Ok, so remove",
                state: orders([order(cid.clone(), ActiveOrderState::from(OpenInFlight))]),
                input: response_cancel_ok(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked Open, response Ok, so remove",
                state: orders([order(cid.clone(), ActiveOrderState::from(open(time_base)))]),
                input: response_cancel_ok(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked CancelInFlight, response Ok, so remove",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::from(CancelInFlight { order: None }),
                )]),
                input: response_cancel_ok(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked OpenInFlight, response Err, so ignore",
                state: orders([order(cid.clone(), ActiveOrderState::from(OpenInFlight))]),
                input: response_cancel_err(cid.clone()),
                expected: orders([order(cid.clone(), ActiveOrderState::from(OpenInFlight))]),
            },
            TestCase {
                name: "tracked Open, response Err, so ignore",
                state: orders([order(cid.clone(), ActiveOrderState::from(open(time_base)))]),
                input: response_cancel_err(cid.clone()),
                expected: orders([order(cid.clone(), ActiveOrderState::from(open(time_base)))]),
            },
            TestCase {
                name: "tracked CancelInFlight w/ Some(Open), response Err, so set Open",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::from(CancelInFlight {
                        order: Some(open(time_base)),
                    }),
                )]),
                input: response_cancel_err(cid.clone()),
                expected: orders([order(cid.clone(), ActiveOrderState::from(open(time_base)))]),
            },
            TestCase {
                name: "tracked CancelInFlight w/ None Open, response Err, so remove",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::from(CancelInFlight { order: None }),
                )]),
                input: response_cancel_err(cid),
                expected: Orders::default(),
            },
        ];

        for mut test in cases.into_iter() {
            test.state.update_from_cancel_response(&test.input);
            assert_eq!(test.state, test.expected, "TC failed: {}", test.name);
        }
    }

    #[test]
    fn test_record_in_flight_cancel() {
        struct TestCase {
            state: Orders<ExchangeId, u64>,
            input: Vec<OrderRequestCancel<ExchangeId, u64>>,
            expected: Orders<ExchangeId, u64>,
        }

        let cid_1 = ClientOrderId::default();
        let cid_2 = ClientOrderId::default();

        let cases = vec![
            TestCase {
                // TC0: Ignore untracked InFlight
                state: Orders::default(),
                input: vec![request_cancel(cid_1.clone())],
                expected: Orders::default(),
            },
            TestCase {
                // TC1: Insert InFlight that is already tracked
                state: orders([order_cancel_in_flight(cid_1.clone())]),
                input: vec![request_cancel(cid_1.clone())],
                expected: orders([order_cancel_in_flight(cid_1.clone())]),
            },
            TestCase {
                // TC2: Ignore one untracked InFlight, and ignore one already tracked
                state: orders([order_cancel_in_flight(cid_1.clone())]),
                input: vec![request_cancel(cid_1.clone()), request_cancel(cid_2.clone())],
                expected: orders([order_cancel_in_flight(cid_1)]),
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
            input: Vec<OrderRequestOpen<ExchangeId, u64>>,
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
