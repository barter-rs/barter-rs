use crate::{
    channel::{Tx, mpsc_unbounded},
    error::SocketError,
    protocol::websocket::{WebSocket, WsError, WsMessage, connect},
    socket::{Message, SocketParser, Transformer, route_manager_events},
};
use futures::StreamExt;

const URL: &str = "wss://streams.fast.onetrading.com";

pub trait AsyncTransformer {
    type Input;
    type Output;
    fn transform(&mut self, input: Self::Input) -> impl Future<Output = Self::Output>;
}

// One connection, ephemeral. Initialised with a reference to OuterSocket State.
pub struct Socket<Stream, Sink> {
    stream: Stream,
    sink: Sink,
}

// Todo:
//  - Not sure if I want init to be async like it currently is
//  Requirements:
//   1. FnInitSocket: AsyncFnMut() -> Result<(Stream, Sink), FnInitSocketError>

struct ReconnectingSocket;

// Dummy State & event types
#[derive(Clone, Debug)]
struct FooState;
#[derive(Clone, Debug)]
struct ManagerEvent(String);
#[derive(Clone, Debug)]
struct StreamEvent(String);

async fn run() {
    let init = |state: &FooState| async {
        let websocket: WebSocket = connect(URL).await?;
        let (sink, stream) = websocket.split();

        // Todo: SocketParser + Transformer -> Message<ManagerEvent, StreamEvent>:
        let stream = stream.map(|ws_result: Result<WsMessage, WsError>| match ws_result {
            Ok(message) => Message::Stream(StreamEvent(message.to_string())),
            Err(error) => Message::Manager(ManagerEvent(error.to_string())),
        });

        Ok::<_, SocketError>((sink, stream))
    };

    let state = FooState;

    let (tx_manager, rx_manager) = mpsc_unbounded::<ManagerEvent>();
    let route = |event: ManagerEvent| async { tx_manager.send(event).map_err(|_| ()) };

    // Initialise Stream<Item = Message<ManagerEvent, StreamEvent>
    let (sink, stream) = init_socket(init, &state).await.unwrap();
}

async fn init_socket<FnInit, FnInitErr, State, Sink, Stream>(
    mut init: FnInit,
    state: &State,
) -> Result<(Sink, Stream), FnInitErr>
where
    FnInit: AsyncFnMut(&State) -> Result<(Sink, Stream), FnInitErr>,
{
    let (sink, stream) = init(state).await?;
    Ok((sink, stream))
}

// async fn init_socket<FnInit, FnInitErr, State, Sink, StreamA, ManagerEvent, StreamEvent, FnRoute>(
//     init: FnInit,
//     state: &State,
//     route: FnRoute,
// ) -> Result<(Sink, impl Stream<Item = StreamEvent), FnInitErr>
// where
//     FnInit: AsyncFnMut(&State) -> Result<(Sink, StreamA), FnInitErr>,
//     FnRoute: AsyncFnMut(ManagerEvent) -> Result<(), ()>,
// {
//     // Initialise Stream<Item = Message<ManagerEvent, StreamEvent>
//     let (sink, stream) = init(state).await.unwrap();
//
//     // Route Message::ManagerEvent to manager, leaving a Stream<Item = StreamEvent>
//     let stream = route_manager_events(
//         stream,
//         route,
//     );
//
//     Ok((sink, stream))
// }
