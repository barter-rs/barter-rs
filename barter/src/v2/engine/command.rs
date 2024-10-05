use crate::v2::execution::ExecutionRequest;
use crate::v2::order::OrderId;
use derive_more::From;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, From)]
pub enum Command<InstrumentKey> {
    // ReSyncEngineState,
    Execute(ExecutionRequest<InstrumentKey>),

    ClosePosition(InstrumentKey),
    CloseAllPositions,

    CancelOrderById((InstrumentKey, OrderId)),
    // CancelOrderByCid((InstrumentKey, ClientOrderId)),
    CancelAllOrders,
}
