use crate::v2::{execution::ExecutionRequest, order::OrderId};
use derive_more::From;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, From)]
pub enum Command<ExchangeKey, InstrumentKey> {
    Execute(ExecutionRequest<ExchangeKey, InstrumentKey>),

    ClosePosition(InstrumentKey),
    CloseAllPositions,

    CancelOrderById((InstrumentKey, OrderId)),
    CancelAllOrders,
}
