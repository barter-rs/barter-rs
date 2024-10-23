use crate::v2::{
    execution::error::ExecutionError,
    order::{
        Cancelled, ClientOrderId, ExchangeOrderState, InternalOrderState, Open, Order,
        RequestCancel, RequestOpen,
    },
    Snapshot,
};
use derive_more::Constructor;
use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};
use std::{collections::hash_map::Entry, fmt::Debug};
use tracing::{debug, error, warn};

pub trait OrderManager<InstrumentKey> {
    fn orders<'a>(&'a self) -> impl Iterator<Item = &'a Order<InstrumentKey, InternalOrderState>>
    where
        InstrumentKey: 'a;
    fn record_in_flight_cancel(&mut self, request: &Order<InstrumentKey, RequestCancel>);
    fn record_in_flight_open(&mut self, request: &Order<InstrumentKey, RequestOpen>);
    fn update_from_open(&mut self, response: &Order<InstrumentKey, Result<Open, ExecutionError>>);
    fn update_from_cancel(
        &mut self,
        response: &Order<InstrumentKey, Result<Cancelled, ExecutionError>>,
    );
    fn update_from_order_snapshot(
        &mut self,
        snapshot: Snapshot<&Order<InstrumentKey, ExchangeOrderState>>,
    );
    fn update_from_opens_snapshot(&mut self, snapshot: Snapshot<&Vec<Order<InstrumentKey, Open>>>);
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct Orders<InstrumentKey>(
    pub FnvHashMap<ClientOrderId, Order<InstrumentKey, InternalOrderState>>,
);

impl<InstrumentKey> OrderManager<InstrumentKey> for Orders<InstrumentKey>
where
    InstrumentKey: Debug + Clone,
{
    fn orders<'a>(&'a self) -> impl Iterator<Item = &'a Order<InstrumentKey, InternalOrderState>>
    where
        InstrumentKey: 'a,
    {
        self.0.values()
    }

    fn record_in_flight_cancel(&mut self, request: &Order<InstrumentKey, RequestCancel>) {
        if let Some(duplicate_cid_order) = self.0.insert(request.cid, Order::from(request)) {
            error!(
                cid = %duplicate_cid_order.cid,
                event = ?duplicate_cid_order,
                "OrderManager upserted Order CancelInFlight with duplicate ClientOrderId"
            );
        }
    }

    fn record_in_flight_open(&mut self, request: &Order<InstrumentKey, RequestOpen>) {
        if let Some(duplicate_cid_order) = self.0.insert(request.cid, Order::from(request)) {
            error!(
                cid = %duplicate_cid_order.cid,
                event = ?duplicate_cid_order,
                "OrderManager upserted Order OpenInFlight with duplicate ClientOrderId"
            );
        }
    }

    fn update_from_open(&mut self, response: &Order<InstrumentKey, Result<Open, ExecutionError>>) {
        match (self.0.entry(response.cid), &response.state) {
            (Entry::Occupied(mut order), Ok(new_open)) => match &order.get().state {
                InternalOrderState::OpenInFlight(_) => {
                    debug!(
                        instrument = ?response.instrument,
                        cid = %response.cid,
                        order = ?order.get(),
                        update = ?response,
                        "OrderManager transitioned Order<OpenInFlight> to Order<Open>"
                    );
                    order.get_mut().state = InternalOrderState::Open(new_open.clone());
                }
                InternalOrderState::Open(existing_open) => {
                    warn!(
                        instrument = ?response.instrument,
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
                    instrument = ?response.instrument,
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
            (Entry::Occupied(order), Err(_)) => match &order.get().state {
                InternalOrderState::OpenInFlight(_) => {
                    warn!(
                        instrument = ?response.instrument,
                        cid = %response.cid,
                        order = ?order.get(),
                        update = ?response,
                        "OrderManager received ExecutionError for Order<OpenInFlight>"
                    );
                    order.remove();
                }
                InternalOrderState::Open(_) => {
                    error!(
                        instrument = ?response.instrument,
                        cid = %response.cid,
                        order = ?order.get(),
                        update = ?response,
                        "OrderManager received ExecutionError for existing Order<Open>"
                    );
                }
                InternalOrderState::CancelInFlight(_) => {
                    error!(
                        instrument = ?response.instrument,
                        cid = %response.cid,
                        ?order,
                        update = ?response,
                        "OrderManager received ExecutionError for existing Order<CancelInFlight>"
                    );
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

    fn update_from_cancel(
        &mut self,
        response: &Order<InstrumentKey, Result<Cancelled, ExecutionError>>,
    ) {
        match (self.0.entry(response.cid), &response.state) {
            (Entry::Occupied(order), Ok(_new_cancel)) => match &order.get().state {
                InternalOrderState::OpenInFlight(_) => {
                    warn!(
                        instrument = ?response.instrument,
                        cid = %response.cid,
                        order = ?order.get(),
                        update = ?response,
                        "OrderManager received Order<Cancelled> Ok response for existing Order<OpenInFlight>"
                    );
                    order.remove();
                }
                InternalOrderState::Open(_) => {
                    warn!(
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
                    instrument = ?response.instrument,
                    cid = %response.cid,
                    update = ?response,
                    "OrderManager received Order<Cancelled> Ok response for untracked ClientOrderId - ignoring"
                );
            }
            (Entry::Occupied(order), Err(_err)) => match &order.get().state {
                InternalOrderState::OpenInFlight(_) => {
                    // Todo: Depends on Err... then fix test
                }
                InternalOrderState::Open(_) => {
                    // Todo: Depends on Err... then fix test
                }
                InternalOrderState::CancelInFlight(_) => {
                    // Todo: Depends on Err... then fix test
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
        snapshot: Snapshot<&Order<InstrumentKey, ExchangeOrderState>>,
    ) {
        todo!()
    }

    fn update_from_opens_snapshot(&mut self, snapshot: Snapshot<&Vec<Order<InstrumentKey, Open>>>) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Todo: paste tests from old versoin
}
