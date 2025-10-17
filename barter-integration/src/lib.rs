#![forbid(unsafe_code)]
#![warn(
    unused,
    clippy::cognitive_complexity,
    unused_crate_dependencies,
    unused_extern_crates,
    clippy::unused_self,
    clippy::useless_let_if_seq,
    missing_debug_implementations,
    rust_2018_idioms,
    rust_2024_compatibility
)]
#![allow(clippy::type_complexity, clippy::too_many_arguments, type_alias_bounds)]

//! # Barter-Integration
//! High-performance, low-level framework for composing flexible web integrations.
//!
//! Utilised by other Barter trading ecosystem crates to build robust financial execution integrations,
//! primarily for public data collection & trade execution. It is:
//! * **Low-Level**: Translates raw data streams communicated over the web into any desired data model using arbitrary data transformations.
//! * **Flexible**: Compatible with any protocol (WebSocket, FIX, Http, etc.), any input/output model, and any user defined transformations.
//!
//! ## Core abstractions:
//! - **RestClient** providing configurable signed Http communication between client & server.
//! - **ExchangeStream** providing configurable communication over any asynchronous stream protocols (WebSocket, FIX, etc.).
//!
//! Both core abstractions provide the robust glue you need to conveniently translate between server & client data models.

use crate::error::SocketError;
use serde::{Deserialize, Serialize};

/// All [`Error`](std::error::Error)s generated in Barter-Integration.
pub mod error;

/// Contains `StreamParser` implementations for transforming communication protocol specific
/// messages into a generic output data structure.
pub mod protocol;

/// Contains the flexible `Metric` type used for representing real-time metrics generically.
pub mod metric;

/// Utilities to assist deserialisation.
pub mod de;

/// Defines a [`SubscriptionId`](subscription::SubscriptionId) new type representing a unique
/// `SmolStr` identifier for a data stream (market data, account data) that has been
/// subscribed to.
pub mod subscription;

/// Defines a trait [`Tx`](channel::Tx) abstraction over different channel kinds, as well as
/// other channel utilities.
///
/// eg/ `UnboundedTx`, `ChannelTxDroppable`, etc.
pub mod channel;

pub mod collection;

/// Stream utilities.
pub mod stream;

pub mod snapshot;

/// [`Validator`]s are capable of determining if their internal state is satisfactory to fulfill
/// some use case defined by the implementor.
pub trait Validator {
    /// Check if `Self` is valid for some use case.
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized;
}

/// [`Transformer`]s are capable of transforming any `Input` into an iterator of
/// `Result<Self::Output, Self::Error>`s.
pub trait Transformer {
    type Error;
    type Input;
    type Output;
    type OutputIter: IntoIterator<Item = Result<Self::Output, Self::Error>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter;
}

/// Determines if something is considered "unrecoverable", such as an unrecoverable error.
///
/// Note that the meaning of [`Unrecoverable`] may vary depending on the context.
pub trait Unrecoverable {
    fn is_unrecoverable(&self) -> bool;
}

/// Trait that communicates if something is terminal (eg/ requires shutdown or restart).
pub trait Terminal {
    fn is_terminal(&self) -> bool;
}

/// Indicates an `Iterator` or `Stream` has ended.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Deserialize, Serialize,
)]
pub struct FeedEnded;
