use crate::SocketError;
use futures::Stream;

/// Contains useful `WebSocket` type aliases and a default `WebSocket` implementation of a
/// [`StreamParser`].
pub mod websocket;

/// Contains HTTP client capable of executing signed & unsigned requests, as well as an associated
/// execution oriented HTTP request.
pub mod http;

/// `StreamParser`s are capable of parsing the input messages from a given stream protocol
/// (eg/ WebSocket, Financial Information eXchange (FIX), etc.) and deserialising into an `Output`.
pub trait StreamParser<Output> {
    type Stream: Stream;
    type Message;
    type Error;

    fn parse(input: Result<Self::Message, Self::Error>) -> Option<Result<Output, SocketError>>;
}
