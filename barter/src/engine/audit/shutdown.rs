use crate::engine::{audit::Audit, error::UnrecoverableEngineError};
use barter_integration::collection::one_or_many::OneOrMany;
use serde::{Deserialize, Serialize};

/// `Engine` shutdown audit.
///
/// Communicates why the `Engine` has shutdown.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum ShutdownAudit<Event> {
    /// Input event feed ended.
    FeedEnded,
    /// `Engine` encountered an `UnrecoverableEngineError` whilst processing an `Event`.
    Error(Event, OneOrMany<UnrecoverableEngineError>),
    /// `Engine` was commanded to shutdown.
    Commanded(Event),
}

impl<State, Event, Output> From<ShutdownAudit<Event>> for Audit<State, Event, Output> {
    fn from(value: ShutdownAudit<Event>) -> Self {
        Self::Shutdown(value)
    }
}
