use crate::engine::{audit::ProcessAudit, error::UnrecoverableEngineError};
use barter_integration::collection::one_or_many::OneOrMany;
use serde::{Deserialize, Serialize};

/// `Engine` shutdown audit.
///
/// Communicates why the `Engine` has shutdown.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum ShutdownAudit<Event, Output> {
    /// Input event feed ended.
    FeedEnded,
    /// `Engine` encountered an `UnrecoverableEngineError` whilst processing an `Event`.
    Error(Event, OneOrMany<UnrecoverableEngineError>),
    /// `Engine` encountered an `UnrecoverableEngineError` after processing an `Event`.
    ErrorWithProcess(
        ProcessAudit<Event, Output>,
        OneOrMany<UnrecoverableEngineError>,
    ),
    /// `Engine` was commanded to shutdown.
    Commanded(Event),
}
