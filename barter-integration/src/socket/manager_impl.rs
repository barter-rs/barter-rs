use crate::{
    AsyncTransformer, TransformerSync,
    protocol::websocket::{WsMessage, WsSink},
    socket::{
        Message, MessageAdmin, MessageAdminApp, MessageAdminWs,
        reconnecting::{
            SocketUpdate,
            sink::{ReconnectingSink, SinkCommand},
        },
    },
};
use futures::{Sink, SinkExt, Stream};
use std::marker::PhantomData;

// Has knowledge of the exchange / business context
// Needs to know how to:
//  1. Construct SinkItems containing payloads the exchange understand (ie/ WsMessage(ApiRequest))
//  2.
pub struct SocketManager<SinkItem> {
    state: State,

    phantom: PhantomData<SinkItem>,
}

// pub enum SocketCommand<Item> {
//     Sink(SinkCommand<Item>)
// }
// pub enum SinkCommand<Item> {
//     Flush,
//     Reconnect,
//     Send(Item)
// }

pub enum SocketCommand {
    Reconnect,
    Subscribe(Subscription),
    Unsubscribe(Subscription),
}

fn run<Sink, Admin, Command>(
    stream: impl Stream<Item = Message<SocketUpdate<Sink, Admin>, Command>>,
) {
    // SocketManager Output stream contains Commands for ReconnectingSink
    // eg/ Message<_, ReconnectingSinkCommand>

    // SocketManager handles "app level" business aware logic
    // ReconnectingSink handles "protocol level" logic & sending

    // SocketManager translates "socket commands" from external, into SinkCommands to achieve some
    // business outcome

    // Ideally SocketManager would be agnostic from the Protocol of the ReconnectingSink, but it
    // would have some generic logic it can use to translate between Business -> Protocol
}

impl<Sink, SinkItem, AdminProtocol> AsyncTransformer for SocketManager<SinkItem> {
    // Todo: Can I / should I make custom types for these Inputs & Outputs, they are crazy
    type Input =
        Message<SocketUpdate<Sink, MessageAdmin<AdminProtocol, MessageAdminApp>>, SocketCommand>;

    // Below is the input for ReconnectingSink, but I need to add Audit too.
    // Output could contain the Input for ReconnectingSink stream Transformer, like below:
    type Output = Message<SocketUpdate<Sink, AdminProtocol>, SinkCommand<SinkItem>>;

    fn transform(&mut self, input: Self::Input) -> impl Future<Output = Self::Output> {
        todo!()
    }
}

pub struct Manager<Sink> {
    sink: ReconnectingSink<Sink>,
    state: State,
}

impl<Sink> AsyncTransformer for Manager<Sink>
where
    Sink: futures::Sink<WsMessage>,
{
    type Input =
        Message<SocketUpdate<Sink, MessageAdmin<MessageAdminWs, MessageAdminApp>>, SocketCommand>;
    type Output = ();

    async fn transform(&mut self, input: Self::Input) -> Self::Output {
        use MessageAdminWs::*;

        // Todo: maybe I just have:
        //  1. dumb ReconnectingSink, handles SocketUpdate<Sink, MessageAdminWs> only
        //  2. clever Controller, handles

        match input {
            Message::Admin(SocketUpdate::Connected(sink)) => {
                if let Some(mut old) = self.sink.swap_sink(sink) {
                    // Todo: add timeout and handle error, maybe even spawn a task since Self will
                    //   likely contain FuturesUnordered<RequestFuture> (see execution.manager.rs)
                    old.flush().await.unwrap();
                    old.close().await.unwrap();
                }
            }
            Message::Admin(SocketUpdate::Reconnecting) => {
                if let Some(mut old) = self.sink.take_sink() {
                    // Todo: same as above
                    old.flush().await.unwrap();
                    old.close().await.unwrap();
                }
            }
            Message::Admin(SocketUpdate::Item(MessageAdmin::Protocol(Ping(ping)))) => {
                // Send Pong(ping) response
            }
            Message::Admin(SocketUpdate::Item(MessageAdmin::Protocol(Pong(pong)))) => {
                // Acknowledge the Pong internally, cancelling timeout future
            }
            Message::Admin(SocketUpdate::Item(MessageAdmin::Protocol(Close(close)))) => {
                // Log and then flush old sink
            }
            Message::Admin(SocketUpdate::Item(MessageAdmin::Protocol(Error(error)))) => {
                // Log, maybe flush sink (error dependent)
            }
            Message::Admin(SocketUpdate::Item(MessageAdmin::Application(application))) => {}
            Message::Payload(SocketCommand::Reconnect) => {
                if let Some(mut old) = self.sink.take_sink() {
                    // Todo: same as above
                    old.flush().await.unwrap();
                    old.close().await.unwrap();
                }
            }
            Message::Payload(SocketCommand::Subscribe(sub)) => {
                // Send Subscription request over the Sink
            }
            Message::Payload(SocketCommand::Unsubscribe(sub)) => {
                // Send Subscription request over the Sink
            }
        }
    }
}

pub struct State {}

pub struct Subscription;

pub enum SubscriptionState {
    SubscribeInFlight,
    Active,
    UnsubscribeInFlight,
}
