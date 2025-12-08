use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum SocketUpdate<Sink, T> {
    Connected(Sink),
    Reconnecting,
    Item(T),
}

impl<Sink, T> From<T> for SocketUpdate<Sink, T> {
    fn from(value: T) -> Self {
        Self::Item(value)
    }
}

impl<Sink, T> SocketUpdate<Sink, T> {
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
