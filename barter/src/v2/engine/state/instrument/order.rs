use crate::v2::{
    engine::state::instrument::OrderManager,
    execution::error::ExecutionError,
    order::{
        CancelInFlight, Cancelled, ClientOrderId, ExchangeOrderState, InternalOrderState, Open,
        OpenInFlight, Order,
    },
    Snapshot,
};
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};
use tracing::{debug, error, warn};
use uuid::Uuid;
use vecmap::{map::Entry, VecMap};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct Orders<InstrumentKey> {
    pub inner: VecMap<ClientOrderId, Order<InstrumentKey, InternalOrderState>>,
}

impl<InstrumentKey> OrderManager<InstrumentKey> for Orders<InstrumentKey>
where
    InstrumentKey: Debug + Display + Clone + PartialEq,
{
    fn record_in_flights(
        &mut self,
        requests: impl IntoIterator<Item = Order<InstrumentKey, OpenInFlight>>,
    ) {
        for request in requests {
            if let Some(duplicate_cid_order) = self.inner.insert(request.cid, Order::from(request))
            {
                error!(
                    cid = %duplicate_cid_order.cid,
                    event = ?duplicate_cid_order,
                    "OrderManager upserted Order<OpenInFlight> with duplicate ClientOrderId"
                );
            }
        }
    }

    fn update_from_open(&mut self, response: &Order<InstrumentKey, Result<Open, ExecutionError>>) {
        match (self.inner.entry(response.cid), &response.state) {
            (Entry::Occupied(mut order), Ok(new_open)) => match &order.get().state {
                InternalOrderState::OpenInFlight(_) => {
                    debug!(
                        instrument = %response.instrument,
                        cid = %response.cid,
                        order = ?order.get(),
                        update = ?response,
                        "OrderManager transitioned Order<OpenInFlight> to Order<Open>"
                    );
                    order.get_mut().state = InternalOrderState::Open(new_open.clone());
                }
                InternalOrderState::Open(existing_open) => {
                    warn!(
                        instrument = %response.instrument,
                        cid = %response.cid,
                        order = ?order.get(),
                        update = ?response,
                        "OrderManager received Order<Open> Ok response for existing Order<Open> - taking latest timestamp"
                    );

                    if new_open.time_update > existing_open.time_update {
                        order.get_mut().state = InternalOrderState::Open(new_open.clone());
                    }
                }
                InternalOrderState::CancelInFlight(_) => {
                    error!(
                        instrument = %response.instrument,
                        cid = %response.cid,
                        order = ?order.get(),
                        update = ?response,
                        "OrderManager received Order<Open> Ok response for existing Order<CancelInFlight>"
                    );
                }
            },
            (Entry::Occupied(mut order), Err(_)) => match &order.get().state {
                InternalOrderState::OpenInFlight(_) => {
                    warn!(
                        instrument = %response.instrument,
                        cid = %response.cid,
                        order = ?order.get(),
                        update = ?response,
                        "OrderManager received ExecutionError for Order<OpenInFlight>"
                    );
                    order.remove();
                }
                InternalOrderState::Open(_) => {
                    error!(
                        instrument = %response.instrument,
                        cid = %response.cid,
                        order = ?order.get(),
                        update = ?response,
                        "OrderManager received ExecutionError for existing Order<Open>"
                    );
                }
                InternalOrderState::CancelInFlight(_) => {
                    error!(
                        instrument = %response.instrument,
                        cid = %response.cid,
                        ?order,
                        update = ?response,
                        "OrderManager received ExecutionError for existing Order<CancelInFlight>"
                    );
                }
            },
            (Entry::Vacant(cid_untracked), Ok(new_open)) => {
                warn!(
                    instrument = %response.instrument,
                    cid = %response.cid,
                    update = ?response,
                    "OrderManager received Order<Open> for untracked ClientOrderId - now tracking"
                );

                cid_untracked.insert(Order::new(
                    response.instrument.clone(),
                    response.cid,
                    response.side,
                    InternalOrderState::from(new_open.clone()),
                ));
            }
            (Entry::Vacant(_), Err(_)) => {
                error!(
                    instrument = %response.instrument,
                    cid = %response.cid,
                    update = ?response,
                    "OrderManager received ExecutionError for untracked ClientOrderId"
                );
            }
        }
    }

    fn update_from_cancel(
        &mut self,
        response: &Order<InstrumentKey, Result<Cancelled, ExecutionError>>,
    ) {
        match &response.state {
            Ok(cancelled) => {
                todo!()
            }
            Err(error) => {
                // Remove from InFlight & log error
                todo!()
            }
        }
    }

    fn update_from_order_snapshot(
        &mut self,
        snapshot: &Snapshot<Order<InstrumentKey, ExchangeOrderState>>,
    ) {
        let Snapshot(snapshot) = snapshot;
        let existing = self.inner.entry(snapshot.cid);

        // Todo: add logging where appropriate below
        // '--> is this robust enough? It's more simple than the previous impl way below
        panic!("todo");

        match &snapshot.state {
            ExchangeOrderState::Open(new_open) => {
                self.inner
                    .entry(snapshot.cid)
                    .and_modify(|order| order.state = InternalOrderState::Open(new_open.clone()))
                    .or_insert(Order::new(
                        snapshot.instrument.clone(),
                        snapshot.cid,
                        snapshot.side,
                        InternalOrderState::Open(new_open.clone()),
                    ));
            }
            ExchangeOrderState::OpenRejected(reason) => {
                if let Some(removed) = self.inner.remove(&snapshot.cid) {
                    // Todo: Log
                }
            }
            ExchangeOrderState::CancelRejected(reason) => {
                if let Some(removed) = self.inner.remove(&snapshot.cid) {
                    // Todo: Log
                }
            }
            ExchangeOrderState::Cancelled(new_cancelled) => {
                if let Some(removed) = self.inner.remove(&snapshot.cid) {
                    // Todo: Log
                }
            }
        }

        // match &order.state {
        //     // Remove InFlight order (if present), and upsert the Open Order
        //     OrderState::Open(open) => {
        //         if let Some(in_flight) = self.in_flights.remove(&order.cid) {
        //             debug!(
        //                 instrument = %order.instrument,
        //                 cid = %order.cid,
        //                 ?in_flight,
        //                 open = ?order,
        //                 "OrderManager removed Order<InFlight> after receiving Snapshot<Order<Open>>"
        //             );
        //         }
        //
        //         if let Some(replaced) = self.opens.insert(
        //             order.cid,
        //             Order::new(
        //                 order.instrument.clone(),
        //                 order.cid,
        //                 order.side,
        //                 open.clone(),
        //             ),
        //         ) {
        //             assert_eq!(
        //                 replaced.instrument, order.instrument,
        //                 "Snapshot<Order> does not have same instrument as existing Order<Open>"
        //             );
        //         }
        //     }
        //     // Remove associated Open (expected), or InFlight (unexpected) order
        //     OrderState::Cancelled(_cancelled) => {
        //         if let Some(open) = self.opens.remove(&order.cid) {
        //             debug!(
        //                 instrument = %order.instrument,
        //                 cid = %order.cid,
        //                 ?open,
        //                 cancel = ?order,
        //                 "OrderManager removed Order<Open> after receiving Snapshot<Order<Cancelled>>"
        //             );
        //         } else if let Some(in_flight) = self.in_flights.remove(&order.cid) {
        //             warn!(
        //                 instrument = %order.instrument,
        //                 cid = %order.cid,
        //                 ?in_flight,
        //                 cancel = ?order,
        //                 "OrderManager removed Order<InFlight> after receiving Snapshot<Order<Cancelled>> - why was this still InFlight?"
        //             );
        //         } else {
        //             warn!(
        //                 instrument = %order.instrument,
        //                 cid = %order.cid,
        //                 cancel = ?order,
        //                 "OrderManager ignoring Snapshot<Order<Cancelled> for un-tracked Order"
        //             );
        //         }
        //     }
        // }
    }
}

impl<InstrumentKey> Default for Orders<InstrumentKey> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::order::OrderId;
    use barter_integration::model::Side;
    use chrono::{DateTime, Utc};
    use std::ops::Add;

    fn specific_open_in_flights(
        orders: impl IntoIterator<Item = Order<u64, OpenInFlight>>,
    ) -> VecMap<ClientOrderId, Order<u64, InternalOrderState>> {
        orders
            .into_iter()
            .map(|order| (order.cid, Order::from(order)))
            .collect()
    }

    fn specific_open_in_flight(cid: ClientOrderId) -> Order<u64, OpenInFlight> {
        Order {
            instrument: 1,
            cid,
            side: Side::Buy,
            state: OpenInFlight,
        }
    }

    fn orders(orders: impl IntoIterator<Item = Order<u64, InternalOrderState>>) -> Orders<u64> {
        Orders {
            inner: orders.into_iter().map(|order| (order.cid, order)).collect(),
        }
    }

    fn open_in_flight(cid: ClientOrderId) -> Order<u64, InternalOrderState> {
        Order {
            instrument: 1,
            cid,
            side: Side::Buy,
            state: InternalOrderState::OpenInFlight(OpenInFlight),
        }
    }

    fn open(
        cid: ClientOrderId,
        id: OrderId,
        secs_since_epoch: i64,
    ) -> Order<u64, InternalOrderState> {
        Order {
            instrument: 1,
            cid,
            side: Side::Buy,
            state: InternalOrderState::Open(Open {
                id,
                time_update: DateTime::<Utc>::MIN_UTC
                    .add(chrono::TimeDelta::seconds(secs_since_epoch)),
                price: 0.0,
                quantity: 0.0,
                filled_quantity: 0.0,
            }),
        }
    }

    fn open_ok(
        cid: ClientOrderId,
        id: OrderId,
        secs_since_epoch: i64,
    ) -> Order<u64, Result<Open, ExecutionError>> {
        Order {
            instrument: 1,
            cid,
            side: Side::Buy,
            state: Ok(Open {
                id,
                time_update: DateTime::<Utc>::MIN_UTC
                    .add(chrono::TimeDelta::seconds(secs_since_epoch)),
                price: 0.0,
                quantity: 0.0,
                filled_quantity: 0.0,
            }),
        }
    }

    fn open_err(cid: ClientOrderId) -> Order<u64, Result<Open, ExecutionError>> {
        Order {
            instrument: 1,
            cid,
            side: Side::Buy,
            state: Err(ExecutionError::X),
        }
    }

    fn cancel_in_flight(cid: ClientOrderId, id: OrderId) -> Order<u64, InternalOrderState> {
        Order {
            instrument: 1,
            cid,
            side: Side::Buy,
            state: InternalOrderState::CancelInFlight(CancelInFlight { id }),
        }
    }

    #[test]
    fn test_record_in_flights() {
        struct TestCase {
            state: Orders<u64>,
            input: Vec<Order<u64, OpenInFlight>>,
            expected: Orders<u64>,
        }

        let cid_1 = ClientOrderId(Uuid::new_v4());
        let cid_2 = ClientOrderId(Uuid::new_v4());

        let cases = vec![
            TestCase {
                // TC0: Insert unseen InFlight
                state: Orders::default(),
                input: vec![specific_open_in_flight(cid_1)],
                expected: Orders {
                    inner: specific_open_in_flights([specific_open_in_flight(cid_1)]),
                },
            },
            TestCase {
                // TC1: Insert InFlight that is already tracked
                state: Orders {
                    inner: specific_open_in_flights([specific_open_in_flight(cid_1)]),
                },
                input: vec![specific_open_in_flight(cid_1)],
                expected: Orders {
                    inner: specific_open_in_flights([specific_open_in_flight(cid_1)]),
                },
            },
            TestCase {
                // TC2: Insert one untracked InFlight, and one already tracked
                state: Orders {
                    inner: specific_open_in_flights([specific_open_in_flight(cid_1)]),
                },
                input: vec![
                    specific_open_in_flight(cid_1),
                    specific_open_in_flight(cid_2),
                ],
                expected: Orders {
                    inner: specific_open_in_flights([
                        specific_open_in_flight(cid_1),
                        specific_open_in_flight(cid_2),
                    ]),
                },
            },
        ];

        for (index, mut test) in cases.into_iter().enumerate() {
            test.state.record_in_flights(test.input);
            assert_eq!(test.state, test.expected, "TestCase {index} failed")
        }
    }

    #[test]
    fn test_update_from_open() {
        struct TestCase {
            state: Orders<u64>,
            input: Order<u64, Result<Open, ExecutionError>>,
            expected: Orders<u64>,
        }

        let cid_1 = ClientOrderId(Uuid::new_v4());
        let order_id_1 = OrderId::new("order_id_1".to_string());

        let cases = vec![
            TestCase {
                // TC0: cid existing OpenInFlight, response Ok(Open)
                state: orders([open_in_flight(cid_1)]),
                input: open_ok(cid_1, order_id_1.clone(), 0),
                expected: orders([open(cid_1, order_id_1.clone(), 0)]),
            },
            TestCase {
                // TC1: cid existing Open, response Ok(Open) w/ older timestamp
                state: orders([open(cid_1, order_id_1.clone(), 1)]),
                input: open_ok(cid_1, order_id_1.clone(), 0),
                expected: orders([open(cid_1, order_id_1.clone(), 1)]),
            },
            TestCase {
                // TC2: cid existing Open, response Ok(Open) w/ newer timestamp
                state: orders([open(cid_1, order_id_1.clone(), 0)]),
                input: open_ok(cid_1, order_id_1.clone(), 1),
                expected: orders([open(cid_1, order_id_1.clone(), 1)]),
            },
            TestCase {
                // TC3: cid existing CancelInFlight, response Ok(Open)
                state: orders([cancel_in_flight(cid_1, order_id_1.clone())]),
                input: open_ok(cid_1, order_id_1.clone(), 1),
                expected: orders([cancel_in_flight(cid_1, order_id_1.clone())]),
            },
            TestCase {
                // TC4: cid untracked, response Ok(Open)
                state: orders([]),
                input: open_ok(cid_1, order_id_1.clone(), 0),
                expected: orders([open(cid_1, order_id_1.clone(), 0)]),
            },
            TestCase {
                // TC5: cid existing OpenInFlight, response Err
                state: orders([open_in_flight(cid_1)]),
                input: open_err(cid_1),
                expected: orders([]),
            },
            TestCase {
                // TC6: cid existing Open, response Err
                state: orders([open(cid_1, order_id_1.clone(), 0)]),
                input: open_err(cid_1),
                expected: orders([open(cid_1, order_id_1.clone(), 0)]),
            },
            TestCase {
                // TC7: cid existing CancelInFlight, response Err
                state: orders([cancel_in_flight(cid_1, order_id_1.clone())]),
                input: open_err(cid_1),
                expected: orders([cancel_in_flight(cid_1, order_id_1.clone())]),
            },
            TestCase {
                // TC8: cid untracked, response Err
                state: orders([]),
                input: open_err(cid_1),
                expected: orders([]),
            },
        ];

        for (index, mut test) in cases.into_iter().enumerate() {
            test.state.update_from_open(&test.input);
            assert_eq!(test.state, test.expected, "TestCase {index} failed")
        }
    }

    #[test]
    fn test_update_from_cancel() {
        todo!()

        // Todo: update these scenarios, they are from update_from_open
        // Scenarios:
        // - InFlight present, Open not-present, response Ok(open)
        // - InFlight present, Open not-present, response Err(open)

        // - InFlight present, Open present, response Ok(open)
        // - InFlight present, Open present, response Err(open)

        // - InFlight not-present, Open not-present, response Ok(open)
        // - InFlight not-present, Open present, response Err(open)
    }

    #[test]
    fn test_update_from_order_snapshot() {
        todo!()
    }
}
