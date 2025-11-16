use crate::{
    error::SocketError, protocol::websocket::{
        connect, process_binary, process_close_frame, process_frame, process_ping, process_pong, process_text,
        WebSocket, WsMessage, WsSink,
    },
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
    Terminate,
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