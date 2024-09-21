use crate::v2::execution::ExecutionRequest;
use derive_more::From;
use serde::{Deserialize, Serialize};
use crate::v2::order::{ClientOrderId, OrderId};

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, From)]
pub enum Command<InstrumentKey> {
    EnableTrading,  // Todo: currently involves state
    DisableTrading, // Todo: currently involves state

    // ReSyncEngineState,
    Execute(ExecutionRequest<InstrumentKey>),


    ClosePosition(InstrumentKey),
    CloseAllPositions,

    CancelOrderById((InstrumentKey, OrderId)),
    CancelOrderByCid((InstrumentKey, ClientOrderId)),
    CancelAllOrders
}
