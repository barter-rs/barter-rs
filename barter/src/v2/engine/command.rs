use derive_more::From;
use serde::{Deserialize, Serialize};
use crate::v2::execution::ExecutionRequest;

#[derive(
    Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, From,
)]
pub enum Command<InstrumentKey> {
    EnableTrading,
    DisableTrading,
    Terminate,
    ReSyncEngineState,
    Execute(ExecutionRequest<InstrumentKey>),
    ClosePosition,
    CloseAllPositions,
}
