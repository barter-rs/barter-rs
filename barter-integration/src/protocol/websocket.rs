use crate::{error::SocketError, protocol::StreamParser};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, connect_async,
    tungstenite::{
        Utf8Bytes,
        client::IntoClientRequest,
        error::ProtocolError,
        protocol::{CloseFrame, frame::Frame},
    },
};
use tracing::debug;

/// Convenient type alias for a tungstenite `WebSocketStream`.
pub type WebSocket = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Convenient type alias for the `Sink` half of a tungstenite [`WebSocket`].
pub type WsSink = futures::stream::SplitSink<WebSocket, WsMessage>;

/// Convenient type alias for the `Stream` half of a tungstenite [`WebSocket`].
pub type WsStream = futures::stream::SplitStream<WebSocket>;

/// Communicative type alias for a tungstenite [`WebSocket`] `Message`.
pub type WsMessage = tokio_tungstenite::tungstenite::Message;

/// Communicative type alias for a tungstenite [`WebSocket`] `Error`.
pub type WsError = tokio_tungstenite::tungstenite::Error;

/// Default [`StreamParser`] implementation for a [`WebSocket`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct WebSocketSerdeParser;

impl<Output> StreamParser<Output> for WebSocketSerdeParser
where
    Output: for<'de> Deserialize<'de>,
{
    type Stream = WebSocket;
    type Message = WsMessage;
    type Error = WsError;

    fn parse(input: Result<Self::Message, Self::Error>) -> Option<Result<Output, SocketError>> {
        match input {
            Ok(ws_message) => match ws_message {
                WsMessage::Text(text) => process_text(text),
                WsMessage::Binary(binary) => process_binary(binary),
                WsMessage::Ping(ping) => process_ping(ping),
                WsMessage::Pong(pong) => process_pong(pong),
                WsMessage::Close(close_frame) => process_close_frame(close_frame),
                WsMessage::Frame(frame) => process_frame(frame),
            },
            Err(ws_err) => Some(Err(SocketError::WebSocket(Box::new(ws_err)))),
        }
    }
}

/// [`StreamParser`] implementation for a [`WebSocket`] that decodes protobuf
/// binary payloads.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct WebSocketProtobufParser;

impl<Output> StreamParser<Output> for WebSocketProtobufParser
where
    Output: prost::Message + Default,
{
    type Stream = WebSocket;
    type Message = WsMessage;
    type Error = WsError;

    fn parse(input: Result<Self::Message, Self::Error>) -> Option<Result<Output, SocketError>> {
        match input {
            Ok(ws_message) => match ws_message {
                WsMessage::Text(payload) => {
                    debug!(?payload, "received Text WebSocket message");
                    None
                }
                WsMessage::Binary(binary) => {
                    Some(Output::decode(binary.as_ref()).map_err(|error| {
                        SocketError::DeserialiseProtobuf {
                            error,
                            payload: binary.to_vec(),
                        }
                    }))
                }
                WsMessage::Ping(ping) => process_ping::<Output>(ping),
                WsMessage::Pong(pong) => process_pong::<Output>(pong),
                WsMessage::Close(close_frame) => process_close_frame::<Output>(close_frame),
                WsMessage::Frame(frame) => process_frame::<Output>(frame),
            },
            Err(ws_err) => Some(Err(SocketError::WebSocket(Box::new(ws_err)))),
        }
    }
}

/// Process a payload of `String` by deserialising into an `ExchangeMessage`.
pub fn process_text<ExchangeMessage>(
    payload: Utf8Bytes,
) -> Option<Result<ExchangeMessage, SocketError>>
where
    ExchangeMessage: for<'de> Deserialize<'de>,
{
    Some(
        serde_json::from_str::<ExchangeMessage>(&payload).map_err(|error| {
            debug!(
                ?error,
                ?payload,
                action = "returning Some(Err(err))",
                "failed to deserialize WebSocket Message into domain specific Message"
            );
            SocketError::Deserialise {
                error,
                payload: payload.to_string(),
            }
        }),
    )
}

/// Process a payload of `Vec<u8>` bytes by deserialising into an `ExchangeMessage`.
pub fn process_binary<ExchangeMessage>(
    payload: Bytes,
) -> Option<Result<ExchangeMessage, SocketError>>
where
    ExchangeMessage: for<'de> Deserialize<'de>,
{
    Some(
        serde_json::from_slice::<ExchangeMessage>(&payload).map_err(|error| {
            debug!(
                ?error,
                ?payload,
                action = "returning Some(Err(err))",
                "failed to deserialize WebSocket Message into domain specific Message"
            );
            SocketError::Deserialise {
                error,
                payload: String::from_utf8(payload.into()).unwrap_or_else(|x| x.to_string()),
            }
        }),
    )
}

/// Basic process for a [`WebSocket`] ping message. Logs the payload at `trace` level.
pub fn process_ping<ExchangeMessage>(ping: Bytes) -> Option<Result<ExchangeMessage, SocketError>> {
    debug!(payload = ?ping, "received Ping WebSocket message");
    None
}

/// Basic process for a [`WebSocket`] pong message. Logs the payload at `trace` level.
pub fn process_pong<ExchangeMessage>(pong: Bytes) -> Option<Result<ExchangeMessage, SocketError>> {
    debug!(payload = ?pong, "received Pong WebSocket message");
    None
}

/// Basic process for a [`WebSocket`] CloseFrame message. Logs the payload at `trace` level.
pub fn process_close_frame<ExchangeMessage>(
    close_frame: Option<CloseFrame>,
) -> Option<Result<ExchangeMessage, SocketError>> {
    let close_frame = format!("{close_frame:?}");
    debug!(payload = %close_frame, "received CloseFrame WebSocket message");
    Some(Err(SocketError::Terminated(close_frame)))
}

/// Basic process for a [`WebSocket`] Frame message. Logs the payload at `trace` level.
pub fn process_frame<ExchangeMessage>(
    frame: Frame,
) -> Option<Result<ExchangeMessage, SocketError>> {
    let frame = format!("{frame:?}");
    debug!(payload = %frame, "received unexpected Frame WebSocket message");
    None
}

/// Connect asynchronously to a [`WebSocket`] server.
pub async fn connect<R>(request: R) -> Result<WebSocket, SocketError>
where
    R: IntoClientRequest + Unpin + Debug,
{
    debug!(?request, "attempting to establish WebSocket connection");
    connect_async(request)
        .await
        .map(|(websocket, _)| websocket)
        .map_err(|error| SocketError::WebSocket(Box::new(error)))
}

/// Determine whether a [`WsError`] indicates the [`WebSocket`] has disconnected.
pub fn is_websocket_disconnected(error: &WsError) -> bool {
    matches!(
        error,
        WsError::ConnectionClosed
            | WsError::AlreadyClosed
            | WsError::Io(_)
            | WsError::Protocol(ProtocolError::SendAfterClosing)
    )
}
