use crate::{
    TransformerSync,
    error::SocketError,
    protocol::websocket::{
        WebSocket, WsError, WsMessage, WsSink, connect, is_websocket_disconnected,
    },
    socket::reconnecting::{
        ReconnectingSocket, StreamUpdate,
        backoff::DefaultBackoff,
        init_reconnecting_socket,
        on_connect_err::{ConnectError, ConnectErrorAction, ConnectErrorKind},
        on_stream_err::StreamErrorAction,
        sink::ReconnectingSink,
    },
};
use futures::{SinkExt, Stream, StreamExt};
use std::marker::PhantomData;
use tokio_stream::Elapsed;
use tokio_tungstenite::tungstenite::{Error, protocol::CloseFrame as WsCloseFrame};
use tracing::{debug, warn};

pub mod reconnecting;

// Todo: Layer Transformations
//  1. Establish connection
//  2. Transform Stream::Item (eg/ WsMessage) to Stream<Item = Message<AdminMessageWs, T>>
//  3. Route WsControlMessage to Manager // actually, can route at end and hide from user transformers
//  4. Transform ProtocolMessage::Data into ApplicationMessage (ie/ ExchangeMessage)
//  5. Transform ApplicationMessage -> enum { Heartbeat, Response, Event }

pub trait Integration {
    type Protocol;
    type Deserialiser;
    type Api;
}

pub enum Message<A, T> {
    Admin(A),
    Data(T),
}

pub enum MessageAdmin<P, A> {
    Protocol(P),
    Application(A),
}

pub enum MessageAdminWs {
    Ping(bytes::Bytes),
    Pong(bytes::Bytes),
    Close(Option<WsCloseFrame>),
    Error(WsError),
}

pub enum MessageAdminApp {
    Heartbeat,
    SubscriptionResponse,
    Error,
}

pub struct WebSocketTransformer;
impl TransformerSync for WebSocketTransformer {
    type Input<'a> = Result<WsMessage, WsError>;
    type Output = Message<MessageAdminWs, bytes::Bytes>;

    fn transform(&mut self, input: Self::Input<'_>) -> Self::Output {
        match input {
            Ok(WsMessage::Text(utf8)) => Message::Data(bytes::Bytes::from(utf8)),
            Ok(WsMessage::Binary(bytes)) => Message::Data(bytes),
            Ok(WsMessage::Frame(frame)) => Message::Data(frame.into_payload()),
            Ok(WsMessage::Ping(bytes)) => Message::Admin(MessageAdminWs::Ping(bytes)),
            Ok(WsMessage::Pong(bytes)) => Message::Admin(MessageAdminWs::Pong(bytes)),
            Ok(WsMessage::Close(close)) => Message::Admin(MessageAdminWs::Close(close)),
            Err(error) => Message::Admin(MessageAdminWs::Error(error)),
        }
    }
}

#[derive(Default)]
pub struct SerdeTransformer<T>(PhantomData<T>);
impl<T> TransformerSync for SerdeTransformer<T> {
    type Input<'a> = bytes::Bytes;
    type Output = Result<T, DeBinaryError>;

    fn transform(&mut self, payload: Self::Input<'_>) -> Self::Output {
        serde_json::from_slice::<T>(&payload).map_err(|error| {
            let payload_str = String::from_utf8(payload.clone().into_vec())
                .unwrap_or_else(|error| error.to_string());

            debug!(
                %error,
                ?payload,
                %payload_str,
                target_type = %std::any::type_name::<T>(),
                "failed to deserialise bytes::Bytes via SerDe"
            );

            DeBinaryError { error, payload }
        })
    }
}
pub struct DeBinaryError {
    pub error: serde_json::error::Error,
    pub payload: bytes::Bytes,
}

#[derive(Default)]
pub struct BinanceTransformer<T>(PhantomData<T>);
pub struct BinanceMessage;

impl<T> TransformerSync for BinanceTransformer<T> {
    type Input<'a> = Result<BinanceMessage, DeBinaryError>;
    type Output = Message<MessageAdminApp, T>; // T => eg/ MarketEvent<Trade>

    fn transform(&mut self, input: Self::Input<'_>) -> Self::Output {
        todo!()
    }
}

const URL: &str = "wss://streams.fast.onetrading.com";
const TIMEOUT_CONNECT: std::time::Duration = std::time::Duration::from_secs(10);
const TIMEOUT_STREAM: std::time::Duration = std::time::Duration::from_secs(10);
#[derive(Clone)]
pub struct Origin(pub String);
pub fn run() {
    let (sink, stream) =
        init_reconnecting_websocket_with_updates(URL, TIMEOUT_CONNECT, TIMEOUT_STREAM);

    // Stream of Streams is flattened from this point, so ability to end inner stream
    // is up to the Manager to orchestrate

    // Todo: Shower Thoughts:
    //  1. ReconnectingStream logic must not exclude "ReconnectingStreams" w/o Sinks
    //      '--> Maybe have two traits, one for Stream, another for Stream<(Sink, Stream)>
    //  2. I'm not sure I need to split the WebSocket so early, which could unlock a lot of the
    //     Stream<(Sink, Stream)> methods :thinking.
}

pub fn init_reconnecting_websocket_with_updates<Output>(
    url: &str,
    timeout_connect: std::time::Duration,
    timeout_stream: std::time::Duration,
) -> (
    ReconnectingSink<WsSink>,
    impl Stream<Item = StreamUpdate<Origin, Output>>,
) {
    let stream = init_reconnecting_websocket(url, timeout_connect, timeout_stream)
        .forward_by(
            |(sink, stream)|
        )
        // .into_reconnecting_sink_and_stream(Origin(URL.to_string()))
}

pub fn init_reconnecting_websocket<Output>(
    url: &str,
    timeout_connect: std::time::Duration,
    timeout_stream: std::time::Duration,
) -> impl Stream<Item = (WsSink, impl Stream<Item = Message<MessageAdmin<MessageAdminWs, MessageAdminApp>, Output>>)>
{
    init_reconnecting_socket(
        || init_websocket_stream(URL, timeout_stream),
        timeout_connect,
        DefaultBackoff,
    )
    .on_connect_err(
        move |err_connect: &ConnectError<SocketError>| match &err_connect.kind {
            ConnectErrorKind::Connect(error) => {
                warn!(
                    %url,
                    %error,
                    action = "reconnecting after backoff",
                    "failed to initialise WebSocket due to connect error"
                );
                ConnectErrorAction::Reconnect
            }
            ConnectErrorKind::Timeout => {
                warn!(
                    %url,
                    timeout = ?timeout_connect,
                    action = "reconnecting after backoff",
                    "failed to initialise WebSocket due to connect timeout"
                );
                ConnectErrorAction::Reconnect
            }
        },
    )
}

pub async fn init_websocket_stream<AppMessage, OutputMessage>(
    url: &str,
    timeout: std::time::Duration,
) -> Result<
    (
        WsSink,
        impl Stream<Item = Message<MessageAdmin<MessageAdminWs, MessageAdminApp>, OutputMessage>>,
    ),
    SocketError,
> {
    let websocket: WebSocket = connect(URL).await?;
    let (sink, stream) = futures::StreamExt::split(websocket);

    let stream = stream.with_timeout(timeout, || {
        warn!(
            %url,
            ?timeout,
            "stream ended due to consecutive event timeout"
        )
    });

    // Todo: Maybe don't filter the Results at this point, let the downstream consumers do it!

    // WebSocketTransformer: Result<WsMessage, WsError> => Message<AdminMessageWs, bytes::Bytes>
    let stream = stream.scan(WebSocketTransformer, |transformer, ws_result| {
        std::future::ready({ Some(transformer.transform(ws_result)) })
    });

    // SerdeTransformer: Message<_, bytes::Bytes> => Message<_, Result<AppMessage, DeBinaryError>>
    let stream = stream.scan(
        SerdeTransformer::<AppMessage>::default(),
        |transformer, message| {
            std::future::ready({
                Some(match message {
                    Message::Admin(admin) => Message::Admin(admin),
                    Message::Data(data) => Message::Data(transformer.transform(data)),
                })
            })
        },
    );

    // AppTransformer: Result<AppMessage, DeBinaryError> -> Message<MessageAdminApp, OutputMessage>
    let stream = stream.scan(
        BinanceTransformer::<OutputMessage>::default(),
        |transformer, message| {
            std::future::ready({
                Some(match message {
                    Message::Admin(protocol) => Message::Admin(MessageAdmin::Protocol(protocol)),
                    Message::Data(data) => match transformer.transform(data) {
                        Message::Admin(app) => Message::Admin(MessageAdmin::Application(app)),
                        Message::Data(data) => Message::Data(data),
                    },
                })
            })
        },
    );

    Ok((sink, stream))
}

// pub fn init_reconnecting_websocket(
//     url: &str,
//     timeout_connect: std::time::Duration,
//     timeout_stream: std::time::Duration,
// ) -> impl Stream<Item = (WsSink, impl Stream<Item = WsMessage>)> {
//     init_reconnecting_socket(
//         || init_websocket_stream(URL, timeout_stream),
//         timeout_connect,
//         DefaultBackoff,
//     )
//     .on_connect_err(
//         move |err_connect: &ConnectError<SocketError>| match &err_connect.kind {
//             ConnectErrorKind::Connect(error) => {
//                 warn!(
//                     %url,
//                     %error,
//                     action = "reconnecting after backoff",
//                     "failed to initialise WebSocket due to connect error"
//                 );
//                 ConnectErrorAction::Reconnect
//             }
//             ConnectErrorKind::Timeout => {
//                 warn!(
//                     %url,
//                     timeout = ?timeout_connect,
//                     action = "reconnecting after backoff",
//                     "failed to initialise WebSocket due to connect timeout"
//                 );
//                 ConnectErrorAction::Reconnect
//             }
//         },
//     )
//     .on_stream_err_filter(move |error: &WsError| {
//         if is_websocket_disconnected(error) {
//             warn!(
//                 %error,
//                 action = "reconnecting after backoff",
//                 "consumed non-recoverable WebSocket error"
//             );
//             StreamErrorAction::Reconnect
//         } else {
//             warn!(
//                 %error,
//                 action = "skipping message",
//                 "consumed recoverable WebSocket error"
//             );
//             StreamErrorAction::Continue
//         }
//     })
// }
