use crate::{
    Message, Transformer,
    error::SocketError,
    protocol::StreamParser,
    serde::{DeBinaryError, SeBinaryError},
};
use futures::{
    Sink, Stream,
    future::Either::{Left, Right},
};
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
use url::Url;

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

/// WebSocket [`Transformer`] that transforms a `WebSocket::Item` into a
/// `Message<AdminWs, bytes::Bytes>`.
#[derive(Debug)]
pub struct WsTransformer;

impl Transformer<Result<WsMessage, WsError>> for WsTransformer {
    type Output<'a> = Message<AdminWs, bytes::Bytes>;

    fn transform<'a>(
        input: Result<WsMessage, WsError>,
    ) -> impl IntoIterator<Item = Self::Output<'a>> + 'a {
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

#[derive(Debug)]
pub enum WsSinkError {
    Tungstenite(WsError),
    SeBinary(SeBinaryError),
}

/// Connect to a [`WebSocket`] server.
pub async fn init_websocket<SeTransf, OutMessage, DeTransf, InMessage>(
    url: &str,
) -> Result<impl Sink<OutMessage> + Stream<Item = Message<AdminWs, InMessage>>, WsError>
where
    SeTransf: for<'a> Transformer<OutMessage, Output<'a> = Result<bytes::Bytes, SeBinaryError>>,
    OutMessage: 'static,
    DeTransf: for<'a> Transformer<bytes::Bytes, Output<'a> = Result<InMessage, DeBinaryError>>,
{
    use futures::{SinkExt, StreamExt, stream};

    // Todo: maybe "init_socket" and have FnConnect parameter to generalise... although this is
    //  coupled due to AdminWs...

    let websocket = connect(url).await?;
    debug!(%url, "successfully connected to WebSocket");

    // Apply Sink pipeline
    let websocket = websocket
        .sink_map_err(WsSinkError::Tungstenite)
        .with_flat_map(|out_message| {
            stream::iter(
                SeTransf::transform(out_message)
                    .into_iter()
                    .map(|result| result.map(WsMessage::Binary).map_err(WsSinkError::SeBinary)),
            )
        });

    // Apply Stream pipeline
    let websocket = websocket
        // Apply WsTransformer: Result<WsMessage, WsError> -> Message<AdminWs, bytes::Bytes>
        .flat_map(|result| stream::iter(WsTransformer::transform(result)))
        // Apply DeTransformer to Message::Payload: Bytes -> Result<ApiMessage, DeBinaryError>
        .flat_map(|message| match message {
            Message::Admin(admin) => Left(stream::iter(std::iter::once(Message::Admin(admin)))),
            Message::Payload(payload) => {
                Right(stream::iter(DeTransf::transform(payload).into_iter().map(
                    |result| match result {
                        Ok(payload) => Message::Payload(payload),
                        Err(error) => Message::Admin(AdminWs::DeError(error)),
                    },
                )))
            }
        });

    Ok(websocket)
}

// pub fn apply_pipeline<DeTransf, ApiMessage, AppTransf, Response, Payload>(
//     websocket: WebSocket,
// ) -> impl Sink<WsMessage> + Stream<Item = Message<AdminWs, MessageApp<Response, Payload>>>
// where
//     DeTransf: for<'a> Transformer<bytes::Bytes, Output<'a> = Result<ApiMessage, DeBinaryError>>,
//     AppTransf: for<'a> Transformer<ApiMessage, Output = MessageApp<Response, Payload>>,
// {
//     use futures::{future::Either::*, StreamExt, stream};
//
//     websocket
//         // Apply WsTransformer: Result<WsMessage, WsError> -> Message<AdminWs, bytes::Bytes>
//         .flat_map(|result| stream::iter(WsTransformer::transform(result)))
//
//         // Apply DeTransformer to Message::Payload: Bytes -> Result<ApiMessage, DeBinaryError>
//         .flat_map(|message| match message {
//             Message::Admin(admin) => Left(
//                 stream::iter(std::iter::once(Message::Admin(admin)))
//             ),
//             Message::Payload(payload) => Right(
//                 stream::iter(DeTransf::transform(payload).into_iter().map(Message::Payload))
//             )
//         })
//
//         // Todo: Maybe returning Stream<Item = Message<AdminWs, ApiMessage> is good enough
//         // Apply AppTransformer: ApiMessage -> MessageApp<Response, Payload>
//         .flat_map(|message| match message {
//             Message::Admin(admin) => Left(
//                 Message::Admin(admin)
//             ),
//             Message::Payload(Err(de_error)) => Left(
//                 Message::Admin(AdminWs::DeError(de_error))
//             ),
//             Message::Payload(Ok(payload)) => Right(
//                 stream::iter(AppTransf::transform(payload).into_iter().map(Message::Payload))
//             )
//         })
// }

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
