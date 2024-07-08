use crate::SocketError;
use futures::Stream;
use serde::de::DeserializeOwned;

/// Contains useful `WebSocket` type aliases and a default `WebSocket` implementation of a
/// [`StreamParser`].
pub mod websocket;

/// Contains HTTP client capable of executing signed & unsigned requests, as well as an associated
/// exchange oriented HTTP request.
pub mod http;

/// `StreamParser`s are capable of parsing the input messages from a given stream protocol
/// (eg/ WebSocket, Financial Information eXchange (FIX), etc.) and deserialising into an `Output`.
pub trait StreamParser {
    type Stream: Stream;
    type Message;
    type Error;

    fn parse<Output>(
        input: Result<Self::Message, Self::Error>,
    ) -> Option<Result<Output, SocketError>>
    where
        Output: DeserializeOwned;
}
