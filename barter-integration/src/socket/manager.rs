use crate::{
    AsyncTransformer,
    channel::{Tx, mpsc_unbounded},
    error::SocketError,
    protocol::websocket::{WsMessage, WsSink},
    socket::{
        BinanceTransformer, Message, MessageAdmin, MessageAdminApp, MessageAdminWs,
        SerdeTransformer, init_reconnecting_socket_with_updates, init_websocket_serde_stream,
        reconnecting::{
            ReconnectingSocket, SocketUpdate, StreamUpdate,
            on_connect_err::{ConnectError, ConnectErrorAction, ConnectErrorKind},
        },
    },
    stream::merge::merge,
};
use futures::{Stream, StreamExt};
use tracing::warn;

// Todo:
//  - Ensure ApiTransformer can raw "ForwardToSink"
//  - Consider Arc<Mutex<State> in Manager for users to set "desired State"
pub struct SocketManager<Sink, State> {
    sink: Sink, // SinkManager containing buffer?
    state: State,
}

// Todo: This is great, but let's first do a hard-coded one
impl<Sink, State, Admin, Command, Audit> AsyncTransformer for SocketManager<Sink, State>
where
    State: AsyncTransformer, // Prefer to use composition over inheritance?
{
    type Input = Message<SocketUpdate<Sink, Admin>, Command>;
    type Output = Audit;

    async fn transform(&mut self, input: Self::Input) -> Self::Output {
        // Todo: SocketUpdate::Connected & Reconnecting update Option<Sink>

        match input {
            Message::Admin(SocketUpdate::Connected(sink)) => {}
            Message::Admin(SocketUpdate::Reconnecting) => {}
            Message::Admin(SocketUpdate::Item(admin)) => {}
            Message::Payload(command) => {}
        }
    }
}

pub struct WebSocketManager<State> {
    sink: WsSink,
    state: State,
}

impl<State, Sub> AsyncTransformer for WebSocketManager<State> {
    type Input = Message<
        SocketUpdate<WsSink, MessageAdmin<MessageAdminWs, MessageAdminApp>>,
        WebSocketCommand<Sub>,
    >;
    type Output = ();

    async fn transform(&mut self, input: Self::Input) -> impl Future<Output = Self::Output> {
        todo!()
    }
}

pub enum WebSocketCommand<Sub> {
    Reconnect,
    Terminate,
    Subscribe(Sub),
    Unsubscribe(Sub),

    // Todo: Component with control of CommandTx could have knowledge of exchange API
    //  '--> So Sub could be provided by that external component and be eg/ BinanceSubRequest
    SendRaw(WsMessage),
}

const URL: &str = "wss://streams.fast.onetrading.com";
const TIMEOUT_CONNECT: std::time::Duration = std::time::Duration::from_secs(10);
const TIMEOUT_STREAM: std::time::Duration = std::time::Duration::from_secs(10);
fn binary() {
    let url = URL;
    let timeout_connect = TIMEOUT_CONNECT;
    let timeout_stream = TIMEOUT_STREAM;
    let manager = SocketManager {
        sink: Option::<WsSink>::None,
    };
    let serde_transformer = SerdeTransformer::default();
    let binance_transformer = BinanceTransformer::default();

    let socket_stream = init_reconnecting_socket_with_updates(
        || init_websocket_serde_stream(URL, serde_transformer, binance_transformer),
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
        || {
            warn!(
                %url,
                timeout = ?timeout_stream,
                "stream ended due to consecutive event timeout"
            )
        },
        timeout_connect,
        timeout_stream,
    );

    let (tx_socket_update, rx_socket_update) = mpsc_unbounded();
    let socket_update_stream = rx_socket_update.into_stream();

    let (tx_command, rx_command) = mpsc_unbounded();
    let command_stream = rx_command.into_stream();

    let (manager_audits, market_stream) = run(
        manager,
        socket_stream,
        socket_update_stream,
        command_stream,
        |update| tx_socket_update.send(update).map_err(|_| ()),
    );
}

pub fn run<Manager, Sink, StreamItem, Admin, Command, Output>(
    manager: Manager,
    socket_stream: impl Stream<Item = SocketUpdate<Sink, StreamItem>>,
    socket_update_stream: impl Stream<Item = SocketUpdate<Sink, Admin>>,
    command_stream: impl Stream<Item = Command>,
    forward_socket_update: impl FnMut(SocketUpdate<Sink, Admin>) -> Result<(), ()>,
) -> (
    impl Stream<Item = Manager::Output>,
    impl Stream<Item = StreamUpdate<Output>>,
)
where
    Manager: AsyncTransformer<Input = Message<SocketUpdate<Sink, Admin>, Command>>,
{
    // Manager Command+SocketUpdate Stream
    let manager_stream = merge(
        socket_update_stream.map(Message::Admin),
        command_stream.map(Message::Payload),
    );

    // Manager Audit Stream (processes Commands + SocketUpdates)
    let manager_audit_stream = manager_stream.scan(manager, |manager, message| {
        manager.transform(message).map(Some)
    });

    let data_stream = socket_stream.forward_by(
        |update| {
            use futures::future::Either::*;
            match update {
                SocketUpdate::Connected(sink) => Left(SocketUpdate::Connected(sink)),
                SocketUpdate::Reconnecting => {
                    // Todo: Manager should get this too, need to Clone
                    Right(StreamUpdate::Reconnecting)
                }
                SocketUpdate::Item(message) => match message {
                    Message::Admin(admin) => Left(SocketUpdate::Item(admin)),
                    Message::Payload(payload) => Right(StreamUpdate::Item(payload)),
                },
            }
        },
        forward_socket_update,
    );

    (manager_audit_stream, data_stream)
}
