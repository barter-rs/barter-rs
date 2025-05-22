use serde::{Deserialize, Serialize};

pub mod stream;

/// [`ReconnectingStream`](stream::ReconnectingStream) `Event` that communicates either `Stream::Item`, or that the inner
/// `Stream` is currently reconnecting.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum Event<Origin, T> {
    /// [`ReconnectingStream`](stream::ReconnectingStream) has disconnecting and is
    /// attempting to reconnect.
    Reconnecting(Origin),
    Item(T),
}

impl<Origin, T> From<T> for Event<Origin, T> {
    fn from(value: T) -> Self {
        Self::Item(value)
    }
}

impl<Origin, T> Event<Origin, T> {
    pub fn map<F, O>(self, op: F) -> Event<Origin, O>
    where
        F: FnOnce(T) -> O,
    {
        match self {
            Event::Reconnecting(origin) => Event::Reconnecting(origin),
            Event::Item(item) => Event::Item(op(item)),
        }
    }
}

impl<Origin, T, E> Event<Origin, Result<T, E>> {
    pub fn map_ok<F, O>(self, op: F) -> Event<Origin, Result<O, E>>
    where
        F: FnOnce(T) -> O,
    {
        match self {
            Event::Reconnecting(origin) => Event::Reconnecting(origin),
            Event::Item(result) => Event::Item(result.map(op)),
        }
    }

    pub fn map_err<F, O>(self, op: F) -> Event<Origin, Result<T, O>>
    where
        F: FnOnce(E) -> O,
    {
        match self {
            Event::Reconnecting(origin) => Event::Reconnecting(origin),
            Event::Item(result) => Event::Item(result.map_err(op)),
        }
    }
}
