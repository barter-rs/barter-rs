use derive_more::From;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From,
)]
pub enum Command {
    Disable,
    Enable,
    Terminate,
    ReSyncEngineState,
}
