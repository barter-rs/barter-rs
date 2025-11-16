use crate::{
    TransformerSync,
    error::SocketError,
    protocol::websocket::{
        WebSocket, WsError, WsMessage, WsSink, connect, is_websocket_disconnected,
    },
    socket::reconnecting::{
        ReconnectingSocket, SocketUpdate, StreamUpdate,
        backoff::DefaultBackoff,
        init_reconnecting_socket,
        on_connect_err::{ConnectError, ConnectErrorAction, ConnectErrorHandler, ConnectErrorKind},
        on_stream_err::StreamErrorAction,
        sink::ReconnectingSink,
    },
};
use futures::{Sink, SinkExt, Stream, StreamExt, stream::SplitSink};
use std::marker::PhantomData;
use tokio_stream::{Elapsed, StreamExt};
use tokio_tungstenite::tungstenite::{Error, protocol::CloseFrame as WsCloseFrame};
use tracing::{debug, warn};

pub mod manager;
pub mod reconnecting;

pub trait Integration {
    // Idea for framework
    type Protocol;
    type Deserialiser;
    type Api;
}

pub enum Message<A, T> {
    Admin(A),
    Payload(T),
}

pub enum MessageAdmin<P, A> {
    Disconnected,
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
    ForwardToSink,
    // Could have: Request + Response and have some RequestId to determine timeouts by
}

pub struct WebSocketTransformer;
impl TransformerSync for WebSocketTransformer {
    type Input<'a> = Result<WsMessage, WsError>;
    type Output = Message<MessageAdminWs, bytes::Bytes>;

    fn transform(&mut self, input: Self::Input<'_>) -> Self::Output {
        match input {
            Ok(WsMessage::Text(utf8)) => Message::Payload(bytes::Bytes::from(utf8)),
            Ok(WsMessage::Binary(bytes)) => Message::Payload(bytes),
            Ok(WsMessage::Frame(frame)) => Message::Payload(frame.into_payload()),
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

pub fn init_reconnecting_socket_with_updates<
    FnConnect,
    ErrConnect,
    FnOnConnectErr,
    FnOnTimeout,
    Socket,
    SinkItem,
    Admin,
    Output,
>(
    connect: FnConnect,
    on_connect_err: FnOnConnectErr,
    on_timeout: FnOnTimeout,
    timeout_connect: std::time::Duration,
    timeout_stream: std::time::Duration,
) -> impl Stream<Item = SocketUpdate<impl Sink<SinkItem>, Socket::Item>>
where
    FnConnect: AsyncFnMut() -> Result<Socket, ErrConnect>,
    FnOnConnectErr: ConnectErrorHandler<ErrConnect>,
    FnOnTimeout: Fn() + 'static,
    Socket: Sink<SinkItem> + Stream<Item = Message<Admin, Output>>,
{
    init_reconnecting_socket(connect, timeout_connect, DefaultBackoff)
        .on_connect_err(on_connect_err)
        .with_socket_updates()
        .with_timeout(timeout_stream, on_timeout)
}

pub async fn init_websocket_stream<Socket, AppMessage, Output>(
    url: &str,
) -> Result<
    impl Sink<WsMessage> + Stream<Item = Message<MessageAdmin<MessageAdminWs, MessageAdminApp>, Output>>,
    SocketError,
> {
    let socket = connect(url)
        .await?
        // WebSocketTransformer: Result<WsMessage, WsError> => Message<AdminMessageWs, bytes::Bytes>
        .scan(WebSocketTransformer, |transformer, ws_result| {
            std::future::ready({ Some(transformer.transform(ws_result)) })
        })
        // SerdeTransformer: Message<_, bytes::Bytes> => Message<_, Result<AppMessage, DeBinaryError>>
        .scan(
            SerdeTransformer::<AppMessage>::default(),
            |transformer, message| {
                std::future::ready({
                    Some(match message {
                        Message::Admin(admin) => Message::Admin(admin),
                        Message::Payload(data) => Message::Payload(transformer.transform(data)),
                    })
                })
            },
        )
        // AppTransformer: Result<AppMessage, DeBinaryError> -> Message<MessageAdminApp, OutputMessage>
        .scan(
            BinanceTransformer::<Output>::default(),
            |transformer, message| {
                std::future::ready({
                    Some(match message {
                        Message::Admin(protocol) => {
                            Message::Admin(MessageAdmin::Protocol(protocol))
                        }
                        Message::Payload(data) => match transformer.transform(data) {
                            Message::Admin(app) => Message::Admin(MessageAdmin::Application(app)),
                            Message::Payload(data) => Message::Payload(data),
                        },
                    })
                })
            },
        );

    Ok(socket)
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
