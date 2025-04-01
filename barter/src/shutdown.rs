use serde::{Deserialize, Serialize};

pub trait SyncShutdown {
    type Result;
    fn shutdown(&mut self) -> Self::Result;
}

pub trait AsyncShutdown {
    type Result;
    fn shutdown(&mut self) -> impl Future<Output = Self::Result>;
}

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Deserialize, Serialize,
)]
pub struct Shutdown;
