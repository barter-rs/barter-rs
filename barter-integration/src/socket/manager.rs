use crate::{
    AsyncTransformer,
    channel::{Tx, mpsc_unbounded},
    protocol::websocket::WsSink,
    socket::{
        Message, MessageAdmin, MessageAdminApp, MessageAdminWs,
        init_reconnecting_websocket_with_updates,
        reconnecting::{SocketUpdate, StreamUpdate},
    },
    stream::merge::merge,
};
use futures::{Stream, StreamExt};
use tokio_stream::StreamExt;

// Todo:
//  - Ensure ApiTransformer can raw "ForwardToSink"

// Todo: remember idea is this is abstracted to work with any State, Command, Admin, etc.
//  eg/ barter-data Subscriptions and/or barter-execution something else
pub struct SocketManager<Sink> {
    sink: Option<Sink>,
}

impl<Sink, Admin, Command, Audit> AsyncTransformer for SocketManager<Sink> {
    type Input = Message<SocketUpdate<Sink, Admin>, Command>;
    type Output = Audit;

    async fn transform(&mut self, input: Self::Input) -> Self::Output {
        match input {
            Message::Admin(SocketUpdate::Connected(sink)) => {}
            Message::Admin(SocketUpdate::Reconnecting) => {}
            Message::Admin(SocketUpdate::Item(admin)) => {}
            Message::Payload(command) => {}
        }
    }
}

const URL: &str = "wss://streams.fast.onetrading.com";
const TIMEOUT_CONNECT: std::time::Duration = std::time::Duration::from_secs(10);
const TIMEOUT_STREAM: std::time::Duration = std::time::Duration::from_secs(10);

pub fn main<Manager, Sink, Admin, Command, Output>(
    manager: Manager,
    command_stream: impl Stream<Item = Command>,
) where
    Manager: AsyncTransformer<Input = Message<SocketUpdate<Sink, Admin>, Command>>,
{
    // Manager (ReconnectingSocket) SocketUpdate channel
    let (tx_socket_update, rx_socket_update) = mpsc_unbounded::<SocketUpdate<Sink, Admin>>();
    let socket_update_stream = rx_socket_update.into_stream();

    // Todo: This is introducing hard-coded WebSocket stuff
    // Central "hot-path" Stream
    let payload_stream = init_reconnecting_websocket_with_updates::<Output>(
        URL,
        TIMEOUT_CONNECT,
        TIMEOUT_STREAM,
        |update| tx_socket_update.send(update).map_err(|_| ()),
    );

    let manager_audit_stream = run_manager(manager, socket_update_stream, command_stream);
}

pub fn run_manager<Manager, Sink, Admin, Command, Output>(
    manager: Manager,
    socket_update_stream: impl Stream<Item = SocketUpdate<Sink, Admin>>,
    command_stream: impl Stream<Item = Command>,
) -> impl Stream<Item = Manager::Output>
where
    Manager: AsyncTransformer<Input = Message<SocketUpdate<Sink, Admin>, Command>>,
{
    // Manager Command+SocketUpdate Stream
    let manager_stream = merge(
        socket_update_stream.map(Message::Admin),
        command_stream.map(Message::Payload),
    );

    // Manager Audit Stream (processes Commands + SocketUpdates)
    manager_stream.scan(manager, |manager, message| {
        std::future::ready(Some(manager.process(message)))
    })
}
