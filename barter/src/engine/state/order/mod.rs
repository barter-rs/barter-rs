use crate::engine::state::order::{
    in_flight_recorder::InFlightRequestRecorder, manager::OrderManager,
};
use barter_execution::order::{
    id::ClientOrderId,
    state::{ActiveOrderState, OrderState},
    Order, RequestCancel, RequestOpen,
};
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
pub struct Orders<ExchangeKey, InstrumentKey>(
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
        use barter_execution::order::state::ActiveOrderState::*;

        let Snapshot(snapshot) = snapshot;

        let (mut current_entry, update) = match (
            self.0.entry(snapshot.cid.clone()),
            snapshot.to_active(),
        ) {
            // Order untracked, input Snapshot is InactiveOrderState (ie/ finished), so ignore
            (Entry::Vacant(_), None) => {
                warn!(
                    exchange = ?snapshot.exchange,
                    instrument = ?snapshot.instrument,
                    strategy = %snapshot.strategy,
                    cid = %snapshot.cid,
                    update = ?snapshot,
                    "OrderManager received inactive order snapshot for untracked order - ignoring"
                );
                return;
            }

            // Order untracked, input Snapshot is ActiveOrderState, so insert
            (Entry::Vacant(entry), Some(update)) => {
                match &update.state {
                    Open(open) if open.quantity_remaining().is_zero() => {
                        debug!(
                            exchange = ?snapshot.exchange,
                            instrument = ?snapshot.instrument,
                            strategy = %snapshot.strategy,
                            cid = %snapshot.cid,
                            update = ?snapshot,
                            "OrderManager ignoring new Open order which is actually FulledFilled"
                        );
                    }
                    _active_order => {
                        debug!(
                            exchange = ?snapshot.exchange,
                            instrument = ?snapshot.instrument,
                            strategy = %snapshot.strategy,
                            cid = %snapshot.cid,
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
                    exchange = ?snapshot.exchange,
                    instrument = ?snapshot.instrument,
                    strategy = %snapshot.strategy,
                    cid = %snapshot.cid,
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
            (OpenInFlight(_), OpenInFlight(_)) => {
                warn!(
                    exchange = ?snapshot.exchange,
                    instrument = ?snapshot.instrument,
                    strategy = %snapshot.strategy,
                    cid = %snapshot.cid,
                    update = ?snapshot,
                    "OrderManager received a duplicate OpenInFlight recording - ignoring"
                );
            }
            (OpenInFlight(_), Open(update)) => {
                debug!(
                    exchange = ?snapshot.exchange,
                    instrument = ?snapshot.instrument,
                    strategy = %snapshot.strategy,
                    cid = %snapshot.cid,
                    update = ?snapshot,
                    "OrderManager transitioned an OpenInFlight order to Open"
                );
                if update.quantity_remaining().is_zero() {
                    current_entry.remove();
                } else {
                    current_entry.get_mut().state = Open(update);
                }
            }
            (OpenInFlight(_), CancelInFlight(update)) => {
                debug!(
                    exchange = ?snapshot.exchange,
                    instrument = ?snapshot.instrument,
                    strategy = %snapshot.strategy,
                    cid = %snapshot.cid,
                    update = ?snapshot,
                    "OrderManager transitioned an OpenInFlight order to CancelInFlight"
                );
                current_entry.get_mut().state = CancelInFlight(update);
            }
            (Open(_), OpenInFlight(_)) => {
                warn!(
                    exchange = ?snapshot.exchange,
                    instrument = ?snapshot.instrument,
                    strategy = %snapshot.strategy,
                    cid = %snapshot.cid,
                    update = ?snapshot,
                    "OrderManager received an OpenInFlight recording for an Open order - ignoring"
                );
            }
            (Open(current), Open(update)) => {
                if current.time_exchange <= update.time_exchange {
                    debug!(
                        exchange = ?snapshot.exchange,
                        instrument = ?snapshot.instrument,
                        strategy = %snapshot.strategy,
                        cid = %snapshot.cid,
                        update = ?snapshot,
                        "OrderManager updating an Open order from a more recent snapshot"
                    );
                    current_entry.get_mut().state = Open(update);
                } else {
                    debug!(
                        exchange = ?snapshot.exchange,
                        instrument = ?snapshot.instrument,
                        strategy = %snapshot.strategy,
                        cid = %snapshot.cid,
                        update = ?snapshot,
                        "OrderManager received an out of sequence Open order snapshot - ignoring"
                    );
                }
            }
            (Open(_), CancelInFlight(update)) => {
                debug!(
                    exchange = ?snapshot.exchange,
                    instrument = ?snapshot.instrument,
                    strategy = %snapshot.strategy,
                    cid = %snapshot.cid,
                    update = ?snapshot,
                    "OrderManager transitioned an Open order to CancelInFlight"
                );
                current_entry.get_mut().state = CancelInFlight(update)
            }
            (CancelInFlight(_), OpenInFlight(_)) => {
                error!(
                    exchange = ?snapshot.exchange,
                    instrument = ?snapshot.instrument,
                    strategy = %snapshot.strategy,
                    cid = %snapshot.cid,
                    update = ?snapshot,
                    "OrderManager received an OpenInFlight recording for a CancelInFlight - ignoring"
                );
            }
            (CancelInFlight(_), Open(_)) => {
                debug!(
                    exchange = ?snapshot.exchange,
                    instrument = ?snapshot.instrument,
                    strategy = %snapshot.strategy,
                    cid = %snapshot.cid,
                    update = ?snapshot,
                    "OrderManager received an Open order snapshot for a CancelInFlight - ignoring"
                );
            }
            (CancelInFlight(_), CancelInFlight(_)) => {
                warn!(
                    exchange = ?snapshot.exchange,
                    instrument = ?snapshot.instrument,
                    strategy = %snapshot.strategy,
                    cid = %snapshot.cid,
                    update = ?snapshot,
                    "OrderManager received a duplicate CancelInFlight recording - ignoring"
                );
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
    use crate::{engine::state::order::Orders, test_utils::time_plus_secs};
    use barter_execution::{
        error::{ConnectivityError, OrderError},
        order::{
            id::{ClientOrderId, OrderId, StrategyId},
            state::{ActiveOrderState, CancelInFlight, Cancelled, Open, OpenInFlight},
            Order, OrderKind, RequestOpen, TimeInForce,
        },
    };
    use barter_instrument::{exchange::ExchangeId, Side};
    use chrono::{DateTime, Utc};
    use rust_decimal_macros::dec;
    use smol_str::SmolStr;

    fn orders(
        orders: impl IntoIterator<Item = Order<ExchangeId, u64, ActiveOrderState>>,
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

    fn order_snapshot_cancelled(
        cid: ClientOrderId,
    ) -> Snapshot<Order<ExchangeId, u64, OrderState<u64, u64>>> {
        Snapshot(Order {
            exchange: ExchangeId::Simulated,
            instrument: 1,
            strategy: StrategyId::unknown(),
            cid,
            side: Side::Buy,
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
            exchange: ExchangeId::Simulated,
            instrument: 1,
            strategy: StrategyId::unknown(),
            cid,
            side: Side::Buy,
            state: OrderState::fully_filled(),
        })
    }

    fn order_snapshot_failed(
        cid: ClientOrderId,
    ) -> Snapshot<Order<ExchangeId, u64, OrderState<u64, u64>>> {
        Snapshot(Order {
            exchange: ExchangeId::Simulated,
            instrument: 1,
            strategy: StrategyId::unknown(),
            cid,
            side: Side::Buy,
            state: OrderState::inactive(OrderError::Connectivity(ConnectivityError::Timeout)),
        })
    }

    fn order_snapshot_expired(
        cid: ClientOrderId,
    ) -> Snapshot<Order<ExchangeId, u64, OrderState<u64, u64>>> {
        Snapshot(Order {
            exchange: ExchangeId::Simulated,
            instrument: 1,
            strategy: StrategyId::unknown(),
            cid,
            side: Side::Buy,
            state: OrderState::expired(),
        })
    }

    fn order_snapshot_open(
        cid: ClientOrderId,
        time_exchange: DateTime<Utc>,
    ) -> Snapshot<Order<ExchangeId, u64, OrderState<u64, u64>>> {
        Snapshot(Order {
            exchange: ExchangeId::Simulated,
            instrument: 1,
            strategy: StrategyId::unknown(),
            cid,
            side: Side::Buy,
            state: OrderState::active(open(time_exchange)),
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

    fn request_cancels(
        orders: impl IntoIterator<Item = Order<ExchangeId, u64, RequestCancel>>,
    ) -> FnvHashMap<ClientOrderId, Order<ExchangeId, u64, ActiveOrderState>> {
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
    ) -> FnvHashMap<ClientOrderId, Order<ExchangeId, u64, ActiveOrderState>> {
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
                        price: dec!(1),
                        quantity: dec!(1),
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
                    ActiveOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
                input: order_snapshot_cancelled(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked CancelInFlight, Snapshot is inactive fully filled, so remove",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
                input: order_snapshot_fully_filled(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked CancelInFlight, Snapshot is inactive failed, so remove",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
                input: order_snapshot_failed(cid.clone()),
                expected: Orders::default(),
            },
            TestCase {
                name: "tracked CancelInFlight, Snapshot is inactive expired, so remove",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { id: None }),
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
                        price: dec!(1),
                        quantity: dec!(1),
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
                    OrderState::active(CancelInFlight { id: None }),
                )),
                expected: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { id: None }),
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
                name: "tracked Open, Snapshot is active CancelInFlight, so update",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::Open(open(time_plus_secs(time_base, 1))),
                )]),
                input: Snapshot(order(
                    cid.clone(),
                    OrderState::active(CancelInFlight { id: None }),
                )),
                expected: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
            },
            TestCase {
                name: "tracked CancelInFlight, Snapshot is active OpenInFlight, so ignore",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
                input: Snapshot(order(cid.clone(), OrderState::active(OpenInFlight))),
                expected: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
            },
            TestCase {
                name: "tracked CancelInFlight, Snapshot is active Open, so ignore",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
                input: order_snapshot_open(cid.clone(), time_base),
                expected: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
            },
            TestCase {
                name:
                    "tracked CancelInFlight, Snapshot is active CancelInFlight, so ignore duplicate",
                state: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
                input: Snapshot(order(
                    cid.clone(),
                    OrderState::active(CancelInFlight { id: None }),
                )),
                expected: orders([order(
                    cid.clone(),
                    ActiveOrderState::CancelInFlight(CancelInFlight { id: None }),
                )]),
            },
        ];

        for mut test in cases.into_iter() {
            test.state.update_from_order_snapshot(test.input.as_ref());
            assert_eq!(test.state, test.expected, "TC failed: {}", test.name)
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
