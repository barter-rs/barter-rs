use std::marker::PhantomData;
use futures::{SinkExt, Stream};
use tokio_stream::{Elapsed, StreamExt};
use tokio_tungstenite::tungstenite::Error;
use tokio_tungstenite::tungstenite::protocol::CloseFrame as WsCloseFrame;
use tracing::{debug, warn};
use crate::error::SocketError;
use crate::protocol::websocket::{connect, is_websocket_disconnected, WebSocket, WsError, WsMessage, WsSink};
use crate::socket::reconnecting::{init_reconnecting_socket, ReconnectingSocket, StreamUpdate};
use crate::socket::reconnecting::backoff::DefaultBackoff;
use crate::socket::reconnecting::on_connect_err::{ConnectError, ConnectErrorAction, ConnectErrorKind};
use crate::socket::reconnecting::on_stream_err::StreamErrorAction;
use crate::socket::reconnecting::sink::ReconnectingSink;
use crate::TransformerSync;

pub mod reconnecting;

// Todo: Layer Transformations
//  1. Establish connection
//  2. Transform Stream::Item (eg/ WsMessage) to Stream<Item = ProtocolMessage>
//  3. Route WsControlMessage to Manager // actually, can route at end and hide from user transformers
//  4. Transform ProtocolMessage::Data into ApplicationMessage (ie/ ExchangeMessage)
//  5. Transform ApplicationMessage -> enum { Heartbeat, Response, Event }

pub enum Message<A, T> {
    Admin(A),
    Data(T)
}

pub enum AdminMessageWs {
    Ping(bytes::Bytes),
    Pong(bytes::Bytes),
    Close(Option<WsCloseFrame>),
}

pub struct WebSocketTransformer;
impl TransformerSync for WebSocketTransformer {
    type Input = WsMessage;
    type Output = Message<AdminMessageWs, bytes::Bytes>;

    fn transform(&mut self, input: Self::Input) -> Self::Output {
        match input {
            WsMessage::Text(utf8) => Message::Data(bytes::Bytes::from(utf8)),
            WsMessage::Binary(bytes) => Message::Data(bytes),
            WsMessage::Frame(frame) => Message::Data(frame.into_payload()),
            WsMessage::Ping(bytes) => Message::Admin(AdminMessageWs::Ping(bytes)),
            WsMessage::Pong(bytes) => Message::Admin(AdminMessageWs::Pong(bytes)),
            WsMessage::Close(close) => Message::Admin(AdminMessageWs::Close(close)),
        }
    }
}

pub struct SerdeTransformer<T>(PhantomData<T>);
impl<T> TransformerSync for SerdeTransformer<T> {
    type Input = bytes::Bytes;
    type Output = Result<T, DeBinaryError>;

    fn transform(&mut self, payload: Self::Input) -> Self::Output {
        serde_json::from_slice::<T>(&payload)
            .map_err(|error| {
                let payload_str = String::from_utf8(payload.clone().into_vec())
                    .unwrap_or_else(|error| error.to_string());

                debug!(
                    %error,
                    ?payload,
                    %payload_str,
                    target_type = %std::any::type_name::<T>(),
                    "failed to deserialise bytes::Bytes via SerDe"
                );

                DeBinaryError {
                    error,
                    payload,
                }
            })
    }
}
pub struct DeBinaryError {
    pub error: serde_json::error::Error,
    pub payload: bytes::Bytes,
}

pub struct BinanceTransformer<T>(PhantomData<T>);
pub struct BinanceMessage;

pub enum ApplicationAdminMessage {
    Heartbeat,
    SubscriptionResponse,
}

impl<T> TransformerSync for BinanceTransformer<T> {
    type Input = BinanceMessage;
    type Output = Message<ApplicationAdminMessage, T>; // T => eg/ MarketEvent<Trade>

    fn transform(&mut self, input: Self::Input) -> Self::Output {
        todo!()
    }
}

const URL: &str = "wss://streams.fast.onetrading.com";
const TIMEOUT_CONNECT: std::time::Duration = std::time::Duration::from_secs(10);
const TIMEOUT_STREAM: std::time::Duration = std::time::Duration::from_secs(10);
#[derive(Clone)] pub struct Origin(pub String);
pub fn run() {
    let (
        sink,
        stream
    ) = init_reconnecting_websocket_with_updates(URL, TIMEOUT_CONNECT, TIMEOUT_STREAM);

    // Stream of Streams is flattened from this point, so ability to end inner stream
    // is up to the Manager to orchestrate

    // Todo: Need to decide where we flatten... don't really want to apply Transformations
    //       to a StreamUpdate :cry
    //       Maybe more logic should exist in the FnConnect? Or I can act on Stream Of Streams
    //       via closure that operates on the InnerStream

    stream
        .map(|x| x)

}

pub fn init_reconnecting_websocket_with_updates(
    url: &str,
    timeout_connect: std::time::Duration,
    timeout_stream: std::time::Duration,
) -> (
    ReconnectingSink<WsSink>,
    impl Stream<Item = StreamUpdate<Origin, WsMessage>>
)
{
    init_reconnecting_websocket(url, timeout_connect, timeout_stream)
        .into_reconnecting_sink_and_stream(Origin(URL.to_string()))
}

pub fn init_reconnecting_websocket(
    url: &str,
    timeout_connect: std::time::Duration,
    timeout_stream: std::time::Duration,
) -> impl Stream<Item = (WsSink, impl Stream<Item = WsMessage>)> {

    init_reconnecting_socket(
        || init_websocket_stream(URL, timeout_stream),
        timeout_connect,
        DefaultBackoff
    ).on_connect_err(move |err_connect: &ConnectError<SocketError>| match &err_connect.kind {
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
    }).on_stream_err_filter(move |error: &WsError| {
        if is_websocket_disconnected(error) {
            warn!(
                %error
                action = "reconnecting after backoff",
                "consumed non-recoverable WebSocket error"
            );
            StreamErrorAction::Reconnect
        } else {
            warn!(
                %error
                action = "skipping message",
                "consumed recoverable WebSocket error"
            );
            StreamErrorAction::Continue
        }
    })
}
pub async fn init_websocket_stream(
    url: &str,
    timeout: std::time::Duration,
) -> Result<(WsSink, impl Stream), SocketError>
{
    let websocket: WebSocket = connect(URL).await?;
    let (sink, stream) = futures::StreamExt::split(websocket);

    let stream = stream
        .with_timeout(timeout, || warn!(
            %url,
            ?timeout,
            "stream ended due to consecutive event timeout"
        ));

    let stream = stream
        .map(|x| )

    Ok((sink, stream))
}



