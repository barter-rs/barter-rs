// High-Level
// Protocol agnostic, but can start with WebSocket
// Abstracts over Request+Response timeouts (eg/ Ping, OpenOrder, etc)
// All "responses" are forwarded to the manager component, where it can maintain state using event sourcing
// State is abstracted, so could be "SubscriptionState" or could be "AccountState"

use futures::{Stream, StreamExt};

// StUpdates: Commands or Events forwarded from Socket
// Sink: Send messages over the Socket
struct SocketManager<State, StUpdates, Sink> {
    state: State,
    updates: StUpdates,
    sink: Sink,

    handle: tokio::task::JoinHandle<()>, // Handle for InnerStream task (eg/ WsStream)
}

enum ManagerUpdate<C, ManagerEvent> {
    Command(Command<C>),
    Event(ManagerEvent),
}

enum Command<T> {
    Terminate,
    Request(T),
}

// Send from From InnerStream task (eg/ WsStream) to SocketManager (event sourcing)
// enum Message<Response, Forward, T> {
//     Response(Response),
//     ForwardToSink(Forward),
//     Data(T),
// }
enum Message<ManagerEvent, StreamEvent> {
    Manager(ManagerEvent),
    Stream(StreamEvent),
}

pub trait StreamParser {
    type Input;
    type Error;
    fn parse<Output>(input: Self::Input) -> Option<Result<Output, Self::Error>>;
}

pub trait SocketParser {
    type Input;
    fn parse<Output>(input: Self::Input) -> impl Iterator<Item = Output>;
}
pub trait Transformer {
    type Input;
    type Output;
    fn transform(input: Self::Input) -> impl IntoIterator<Item = Self::Output>;
}

// fn run_outer() {
//     let route = |message| {
//         println!("Routing: {message:?}");
//         std::future::ready(Ok::<(), String>(()))
//     };
// }

fn run<Parser, Transform, ManagerEvent, StreamEvent, FnRoute>(
    stream: impl Stream<Item = Parser::Input> + Unpin + 'static,
    route: FnRoute,
) -> impl Stream<Item = StreamEvent>
where
    Parser: SocketParser + 'static,
    Transform: Transformer<Output = Message<ManagerEvent, StreamEvent>> + 'static,
    FnRoute: AsyncFnMut(ManagerEvent) -> Result<(), ()>,
{
    let stream = parse_protocol_stream_and_transform::<Parser, Transform>(stream);
    let stream = route_manager_events::<ManagerEvent, StreamEvent, FnRoute>(stream, route);

    stream
}

fn parse_protocol_stream_and_transform<Parser, Transform>(
    stream: impl Stream<Item = Parser::Input>,
) -> impl Stream<Item = Transform::Output>
where
    Parser: SocketParser,
    Transform: Transformer,
{
    stream
        // 1. SocketParser into Transformer::Input
        .flat_map(|item| futures::stream::iter(Parser::parse::<Transform::Input>(item)))
        // 2. Transformer::Input to Transformer::Output
        .flat_map(|input| futures::stream::iter(Transform::transform(input)))
}

fn route_manager_events<ManagerEvent, StreamEvent, FnRoute>(
    stream: impl Stream<Item = Message<ManagerEvent, StreamEvent>> + Unpin + 'static,
    route: FnRoute,
) -> impl Stream<Item = StreamEvent>
where
    FnRoute: AsyncFnMut(ManagerEvent) -> Result<(), ()>,
{
    futures::stream::unfold((stream, route), |(mut stream, mut route)| async move {
        let message = stream.next().await?;
        match message {
            Message::Manager(manager_event) => {
                if route(manager_event).await.is_err() {
                    None
                } else {
                    Some((None, (stream, route)))
                }
            }
            Message::Stream(stream_event) => Some((Some(stream_event), (stream, route))),
        }
    })
    .filter_map(std::future::ready)
}
