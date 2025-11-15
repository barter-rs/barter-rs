use crate::{
    error::SocketError, protocol::websocket::{
        connect, process_binary, process_close_frame, process_frame, process_ping, process_pong, process_text,
        WebSocket, WsMessage, WsSink,
    },
    socket_old::retry_socket::{init_reconnecting_socket, ReconnectingSocket, StreamEvent},
    AsyncTransformer,
    TransformerSync,
};
use futures::{Stream, StreamExt};
use std::marker::PhantomData;
use tokio_tungstenite::tungstenite::Utf8Bytes;
use tracing::debug;
use crate::socket::reconnecting::backoff::DefaultBackoff;
use crate::socket::reconnecting::{init_reconnecting_socket, ReconnectingSocket};
use crate::socket::reconnecting::on_connect_err::ConnectError;
use crate::socket::reconnecting::sink::ReconnectingSink;

const URL: &str = "wss://streams.fast.onetrading.com";
const TIMEOUT_CONNECT: std::time::Duration = std::time::Duration::from_secs(10);
const TIMEOUT_STREAM: std::time::Duration = std::time::Duration::from_secs(10);

pub trait SocketParser {
    type Input;
    type Error;
    fn parse(input: Self::Input) -> Option<Result<SocketPayload, Self::Error>>;
}

pub enum SocketPayload {
    Text(Utf8Bytes),
    Binary(bytes::Bytes),
}

pub struct WebSocketParser;

impl SocketParser for WebSocketParser {
    type Input = WsMessage;
    type Error = SocketError;
    fn parse(input: Self::Input) -> Option<Result<SocketPayload, Self::Error>> {
        match input {
            WsMessage::Text(text) => Some(Ok(SocketPayload::Text(text))),
            WsMessage::Binary(binary) => Some(Ok(SocketPayload::Binary(binary))),
            WsMessage::Ping(ping) => {
                debug!(payload = ?ping, "received WebSocket Ping message");
                None
            }
            WsMessage::Pong(pong) => {
                debug!(payload = ?pong, "received WebSocket Pong message");
                None
            }
            WsMessage::Close(close_frame) => {
                debug!(payload = ?close_frame, "received WebSocket CloseFrame message");
                None
            }
            WsMessage::Frame(frame) => {
                debug!(payload = ?frame, "received unexpected WebSocket Frame message");
                None
            }
        }
    }
}

pub struct WebSocketManager {
    sink: ReconnectingSink<WsSink>,
}

pub struct Subscription;

pub enum WebSocketManagerInput<SinkItem> {
    Tick, // For now, just merge input Stream with IntervalStream<1 second> for easy wakeup
    Command(SubscriptionCommand),
    Reconnecting,
    FromStream(WebSocketManagerEvent<SinkItem>),
}

pub enum SubscriptionCommand {
    Subscribe(Subscription),
    Unsubscribe(Subscription),
}

pub struct WebSocketManagerOutput;

impl AsyncTransformer for WebSocketManager {
    type Input = WebSocketManagerInput<WsMessage>;
    type Output = WebSocketManagerOutput;

    async fn transform(&mut self, input: Self::Input) -> Self::Output {
        match input {
            WebSocketManagerInput::Tick => {
                // Check for request-response timeouts (eg/ SubRequest, or PingPong, etc).
            }
            WebSocketManagerInput::Command(SubscriptionCommand::Subscribe(_sub)) => {}
            WebSocketManagerInput::Command(SubscriptionCommand::Unsubscribe(_sub)) => {}
            WebSocketManagerInput::Pong(_) => {}
            WebSocketManagerInput::Response(_) => {}
            WebSocketManagerInput::Reconnecting => {
                // Indicates all Subscriptions have been wiped
                // Problem -> init_socket may already be setting up subscriptions?
            }
        }

        todo!()
    }
}

pub struct StreamTransformer<T> {
    phantom: PhantomData<T>,
}

pub enum StreamTransformerOutput<T> {
    Market(T),
    Manager(WebSocketManagerEvent<WsMessage>),
}

pub enum WebSocketManagerEvent<SinkItem> {
    ForwardToSink(SinkItem),
    Response(Result<Subscription, Subscription>),
}

// Todo: it is pleasant the framework handling ProtocolMessage -> impl Iterator<ExchangeMessage>
//  '--> it would be a shame to lose that.
//  '--> but for now we shall pretend the StreamTransformer goes SocketPayload => MarketEvent
impl<T> TransformerSync for StreamTransformer<T> {
    type Input = SocketPayload;
    type Output = StreamTransformerOutput<T>;

    fn transform(&mut self, input: Self::Input) -> Self::Output {
        // Todo: Generic over T doesn't make sense since at this point it's very exchange API specific
        //  '--> Transformer here needs to understand what messages need to go to the Manager/Sink.
        //  So need understanding of ExchangeMessage (could be T, tbf), as well as routing
        //  requirements
        todo!()
    }
}

pub fn init_reconnecting_websocket(
    url: &str,
    timeout_connect: std::time::Duration,
    timeout_stream: std::time::Duration,
) -> (
    ReconnectingSink<WsSink>,
    impl Stream<Item = StreamEvent<String, WsMessage>>,
) {
    let fn_connect = || async {
        let websocket: WebSocket = connect(URL).await?;

        let (sink, stream) = futures::StreamExt::split(websocket);

        // Todo: maybe ReconnectingSocket method is better, since we can end the inner Stream upon
        //       timeout.
        let stream =
            tokio_stream::StreamExt::timeout(stream, TIMEOUT_STREAM).map(|result| match result {
                Ok(item) => Some(item),
            });
        Ok::<_, SocketError>((sink, stream))
    };

    let origin = url.to_string();

    init_reconnecting_socket(fn_connect, timeout_connect, DefaultBackoff)
        // .on_connect_err(move |err_connect: &ConnectError<SocketError>| todo!())
        // .on_stream_err_filter(move |err_stream: &WsError| todo!())
        .map(|result| result.unwrap())
        .into_reconnecting_sink_and_stream(origin)
}

pub async fn run() {
    let (sink, stream) = init_reconnecting_websocket(URL, TIMEOUT_CONNECT, TIMEOUT_STREAM);

    let stream = stream
        .filter_map(|stream_event| match stream_event {
            StreamEvent::Item(event) => Some(event),
            StreamEvent::Reconnecting(_) => None,
        })
        .filter_map(|event| WebSocketParser::parse(event).transpose().ok().flatten());
}

// fn run<St, Transformer>(
//     stream: St,
//     transformer: Transformer,
// ) -> impl Stream<Item = Transformer::Output>
// where
//     St: Stream,
//     Transformer: AsyncTransformer
// {
//     stream
//         .scan(transformer, |transformer, input| Some(transformer.transform(input)))
// }
