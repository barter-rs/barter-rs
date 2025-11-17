use crate::{
    TransformerSync,
    error::SocketError,
    protocol::websocket::{WsError, WsMessage, connect},
    socket::reconnecting::{
        ReconnectingSocket, SocketUpdate, backoff::DefaultBackoff, init_reconnecting_socket,
        on_connect_err::ConnectErrorHandler,
    },
};
use futures::{Sink, Stream, StreamExt};
use serde::Deserialize;
use std::marker::PhantomData;
use tokio_tungstenite::tungstenite::protocol::CloseFrame as WsCloseFrame;
use tracing::debug;

pub mod manager;
mod manager_impl;
pub mod reconnecting;

pub enum Message<A, T> {
    Admin(A),
    Payload(T),
}

pub enum MessageAdmin<P, A> {
    Protocol(P),
    Application(A),
}

pub enum MessageAdminWs {
    Ping(bytes::Bytes), // don't really need pings and pongs
    Pong(bytes::Bytes),
    Close(Option<WsCloseFrame>), // This & error could be logged & then mapped to Command::Close
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

pub struct SerdeTransformer<T>(PhantomData<T>);

impl<T> Default for SerdeTransformer<T> {
    fn default() -> Self {
        Self(<_>::default())
    }
}

impl<T> TransformerSync for SerdeTransformer<T>
where
    T: for<'de> serde::Deserialize<'de>,
{
    type Input<'a> = bytes::Bytes;
    type Output = Result<T, DeBinaryError>;

    fn transform(&mut self, payload: Self::Input<'_>) -> Self::Output {
        serde_json::from_slice::<T>(&payload).map_err(|error| {
            let payload_str = String::from_utf8(payload.clone().to_vec())
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

pub struct BinanceTransformer<T>(PhantomData<T>);

impl<T> Default for BinanceTransformer<T> {
    fn default() -> Self {
        Self(<_>::default())
    }
}

#[derive(Debug, Deserialize)]
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
    ConnectErr,
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
    FnConnect: AsyncFnMut() -> Result<Socket, ConnectErr>,
    FnOnConnectErr: ConnectErrorHandler<ConnectErr>,
    FnOnTimeout: Fn() + 'static,
    Socket: Sink<SinkItem> + Stream<Item = Message<Admin, Output>>,
{
    init_reconnecting_socket(connect, timeout_connect, DefaultBackoff)
        .on_connect_err(on_connect_err)
        // Todo: Add timeouts before SOS.flatten()
        // .map(|socket| {
        //     let (sink, stream) = socket.split();
        //     let stream = stream.with_timeout(timeout_stream, on_timeout);
        //     sink.reunite(stream).unwrap()
        // })
        .with_socket_updates()
}

pub async fn init_websocket(
    url: &str,
) -> Result<impl Sink<WsMessage> + Stream<Item = Message<MessageAdminWs, bytes::Bytes>>, SocketError>
{
    let socket = connect(url).await?;

    Ok(socket.scan(WebSocketTransformer, |transformer, ws_result| {
        std::future::ready(Some(transformer.transform(ws_result)))
    }))
}

pub async fn init_websocket_serde_stream<
    Socket,
    DeTransformer,
    AppTransformer,
    AppMessage,
    OutputMessage,
>(
    url: &str,
    de_transformer: DeTransformer,
    app_transformer: AppTransformer,
) -> Result<
    impl Sink<WsMessage>
    + Stream<Item = Message<MessageAdmin<MessageAdminWs, MessageAdminApp>, OutputMessage>>,
    SocketError,
>
where
    DeTransformer: for<'a> TransformerSync<
            Input<'a> = bytes::Bytes,
            Output = Result<AppMessage, DeBinaryError>,
        >,
    AppTransformer: for<'a> TransformerSync<
            Input<'a> = DeTransformer::Output,
            Output = Message<MessageAdminApp, OutputMessage>,
        >,
    AppMessage: for<'de> serde::Deserialize<'de>,
{
    let socket = connect(url)
        .await?
        .scan(WebSocketTransformer, |transformer, ws_result| {
            std::future::ready(Some(transformer.transform(ws_result)))
        })
        .scan(de_transformer, |transformer, message| {
            std::future::ready({
                Some(match message {
                    Message::Admin(admin) => Message::Admin(admin),
                    Message::Payload(data) => Message::Payload(transformer.transform(data)),
                })
            })
        })
        .scan(app_transformer, |transformer, message| {
            std::future::ready({
                Some(match message {
                    Message::Admin(protocol) => Message::Admin(MessageAdmin::Protocol(protocol)),
                    Message::Payload(data) => match transformer.transform(data) {
                        Message::Admin(app) => Message::Admin(MessageAdmin::Application(app)),
                        Message::Payload(data) => Message::Payload(data),
                    },
                })
            })
        });

    Ok(socket)
}
