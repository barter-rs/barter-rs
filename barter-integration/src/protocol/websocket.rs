use crate::{
    Message, Transformer,
    error::SocketError,
    protocol::StreamParser,
    serde::{de::DeBinaryError, se::SeError},
};
use futures::{Sink, Stream};
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

/// WebSocket administration message variants.
#[derive(Debug)]
pub enum AdminWs {
    Ping(bytes::Bytes),
    Pong(bytes::Bytes),
    Close(Option<CloseFrame>),
    WsError(WsError),
    DeError(DeBinaryError),
}

#[derive(Debug, thiserror::Error)]
pub enum WsSinkError {
    #[error("tungstenite: {0}")]
    Tungstenite(WsError),
    #[error("serialisation: {0}")]
    Serialisation(SeError),
}

/// WebSocket [`Transformer`] that transforms a `WebSocket::Item` into a
/// `Message<AdminWs, bytes::Bytes>`.
#[derive(Debug)]
pub struct WsTransformer;

impl Transformer<Result<WsMessage, WsError>> for WsTransformer {
    type Output<'a> = Message<AdminWs, bytes::Bytes>;

    fn transform<'a>(
        input: Result<WsMessage, WsError>,
    ) -> impl IntoIterator<Item = Self::Output<'a>, IntoIter: Send> + 'a {
        let output = match input {
            Ok(WsMessage::Text(utf8)) => Message::Payload(bytes::Bytes::from(utf8)),
            Ok(WsMessage::Binary(bytes)) => Message::Payload(bytes),
            Ok(WsMessage::Frame(frame)) => Message::Payload(frame.into_payload()),
            Ok(WsMessage::Ping(bytes)) => Message::Admin(AdminWs::Ping(bytes)),
            Ok(WsMessage::Pong(bytes)) => Message::Admin(AdminWs::Pong(bytes)),
            Ok(WsMessage::Close(close)) => Message::Admin(AdminWs::Close(close)),
            Err(error) => Message::Admin(AdminWs::WsError(error)),
        };

        std::iter::once(output)
    }
}

/// Connect to a [`WebSocket`] server.
pub async fn init_websocket<Serialise, OutMessage, SinkItem, Deserialise, InMessage>(
    url: &str,
) -> Result<
    impl Sink<OutMessage, Error = WsSinkError> + Stream<Item = Message<AdminWs, InMessage>> + Send,
    WsError,
>
where
    Serialise: for<'a> Transformer<OutMessage, Output<'a> = Result<SinkItem, SeError>>,
    SinkItem: Into<WsMessage>,
    for<'a> OutMessage: Debug + 'a,
    Deserialise: for<'a> Transformer<bytes::Bytes, Output<'a> = Result<InMessage, DeBinaryError>>,
    InMessage: Send,
{
    use futures::{
        SinkExt, StreamExt,
        future::Either::{Left, Right},
        stream,
    };

    let websocket = connect(url).await?;
    debug!(%url, "successfully connected to WebSocket");

    // Apply Sink pipeline
    let websocket = websocket
        .sink_map_err(WsSinkError::Tungstenite)
        .with_flat_map(move |out_message| {
            debug!(
                payload = ?out_message,
                target_type = "bytes::Bytes",
                "serialising OutMessage before sending via Sink"
            );

            stream::iter(Serialise::transform(out_message).into_iter().map(|result| {
                result
                    .map(SinkItem::into)
                    .map_err(WsSinkError::Serialisation)
            }))
        });

    // Apply Stream pipeline
    let websocket = websocket
        // WsTransformer: Result<WsMessage, WsError> -> Message<AdminWs, bytes::Bytes>
        .flat_map(|result| stream::iter(WsTransformer::transform(result)))
        // Deserialise Message::Payload: Bytes -> Result<ApiMessage, DeBinaryError>
        .flat_map(|message| match message {
            Message::Admin(admin) => Left(stream::iter(std::iter::once(Message::Admin(admin)))),
            Message::Payload(payload) => Right(stream::iter(
                Deserialise::transform(payload)
                    .into_iter()
                    .map(|result| match result {
                        Ok(payload) => Message::Payload(payload),
                        Err(error) => Message::Admin(AdminWs::DeError(error)),
                    }),
            )),
        });

    Ok(websocket)
}

/// Connect asynchronously to a [`WebSocket`] server.
pub async fn connect<R>(request: R) -> Result<WebSocket, WsError>
where
    R: IntoClientRequest + Unpin + Debug,
{
    debug!(url = ?request, "attempting to establish WebSocket connection");
    connect_async(request).await.map(|(websocket, _)| websocket)
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
        println!("{input:?}");
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
    payload: bytes::Bytes,
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
pub fn process_ping<ExchangeMessage>(
    ping: bytes::Bytes,
) -> Option<Result<ExchangeMessage, SocketError>> {
    debug!(payload = ?ping, "received Ping WebSocket message");
    None
}

/// Basic process for a [`WebSocket`] pong message. Logs the payload at `trace` level.
pub fn process_pong<ExchangeMessage>(
    pong: bytes::Bytes,
) -> Option<Result<ExchangeMessage, SocketError>> {
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
