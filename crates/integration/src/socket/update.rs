use serde::{Deserialize, Serialize};

/// Socket lifecycle events wrapping stream items.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum SocketUpdate<Sink, T> {
    /// Socket connected, providing the sink for sending data.
    Connected(Sink),
    /// Socket disconnected, reconnection in progress.
    Reconnecting,
    /// Data item received from the socket.
    Item(T),
}

impl<Sink, T> From<T> for SocketUpdate<Sink, T> {
    fn from(value: T) -> Self {
        Self::Item(value)
    }
}

impl<Sink, T> SocketUpdate<Sink, T> {
    /// Maps the item using the provided function.
    pub fn map<F, O>(self, op: F) -> SocketUpdate<Sink, O>
    where
        F: FnOnce(T) -> O,
    {
        match self {
            SocketUpdate::Connected(sink) => SocketUpdate::Connected(sink),
            SocketUpdate::Reconnecting => SocketUpdate::Reconnecting,
            SocketUpdate::Item(item) => SocketUpdate::Item(op(item)),
        }
    }
}

impl<Sink, T, E> SocketUpdate<Sink, Result<T, E>> {
    /// Maps the Ok value of a Result item.
    pub fn map_ok<F, O>(self, op: F) -> SocketUpdate<Sink, Result<O, E>>
    where
        F: FnOnce(T) -> O,
    {
        match self {
            SocketUpdate::Connected(sink) => SocketUpdate::Connected(sink),
            SocketUpdate::Reconnecting => SocketUpdate::Reconnecting,
            SocketUpdate::Item(result) => SocketUpdate::Item(result.map(op)),
        }
    }

    /// Maps the Err value of a Result item.
    pub fn map_err<F, O>(self, op: F) -> SocketUpdate<Sink, Result<T, O>>
    where
        F: FnOnce(E) -> O,
    {
        match self {
            SocketUpdate::Connected(sink) => SocketUpdate::Connected(sink),
            SocketUpdate::Reconnecting => SocketUpdate::Reconnecting,
            SocketUpdate::Item(result) => SocketUpdate::Item(result.map_err(op)),
        }
    }
}
