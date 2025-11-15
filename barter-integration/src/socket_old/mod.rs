// High-Level
// Protocol agnostic, but can start with WebSocket
// Abstracts over Request+Response timeouts (eg/ Ping, OpenOrder, etc)
// All "responses" are forwarded to the manager component, where it can maintain state using event sourcing
// State is abstracted, so could be "SubscriptionState" or could be "AccountState"

mod friday;
mod new;
mod retry_socket;

use fnv::FnvHashMap;

/// This is essentially a paradigm that allows you to manage a connection to an external component
///
/// Manage the socket_old an external component. Speaks the language of the external components
/// public API (eg/ BinanceMessage). Enables request-response operations as well as Streaming data.
///
///
///
/// GodSocket: Are we essentially making the Socket but where the Sink is the SocketManager?
/// This feels higher level, perhaps at the client level - nothing to do with State.
/// struct Socket<State, Stream, Sink> {
///     state: Arc<State>,
///     stream: Stream,
///     sink: Sink,
/// }
///
/// Composition over inheritance, but can contain as well as implement? self.sink is better
/// impl<State> Sink for Socket<State>
pub struct SocketManager<MarketState, AccountState, Updates, Sink> {
    market_data: FnvHashMap<u64, Socket<MarketState, Updates, Sink>>,
    account_data: FnvHashMap<u64, Socket<AccountState, Updates, Sink>>,
}

// StUpdates: Commands or Events forwarded from Socket
// Sink: Send messages over the Socket
pub struct Socket<State, Updates, Sink> {
    /// State relating to the Socket.
    ///
    /// Could be SubscriptionState, AccountState, MarketSnapshot (or even OrderBookState?)
    pub state: State, // Since Manager isn't perf sensitive, can read State from outside
    pub updates: Updates,
    pub sink: Sink,
}
//
// impl<State, Updates, Sink> Socket<State, Updates, Sink> {}
//
// /// Defines how a component processing an input Event and generates an appropriate Audit.
// // pub trait Processor<Event> {
// pub trait Processor {
//     type Event;
//     type Audit;
//     fn process(&mut self, event: Self::Event) -> Self::Audit;
// }
//
// pub fn run<State, Updates, Sink>(state: State, mut updates: Updates, sink: Sink)
// // ) -> impl Stream<Item = State::Audit>
// where
//     State: Processor,
//     Updates: Stream<Item = State::Event>,
// {
//     let audits = updates.scan(state, |state, update| {
//         std::future::ready(Some(state.process(update)))
//     });
// }
//
// impl<State, Updates, Sink> Socket<State, Updates, Sink> {
//     async fn run(self)
//     where
//         State: Processor,
//         Updates: Stream<Item = State::Event> + Unpin,
//     {
//         let Self {
//             mut state,
//             mut updates,
//             sink,
//         } = self;
//
//         let audits = updates.scan(state, |state, update| {
//             std::future::ready(Some(state.process(update)))
//         });
//     }
// }
//
// /// Remember user API needs to be ergonomic, perhaps user can mutate &SocketManager
// /// using an internal Arc<Mutex<State>> - NO.
// ///
// /// fn mutate_state(&self, ) -> impl Future<Output = Result<(), Error>> // represents send + validation
//
// pub enum ManagerUpdate<C, ManagerEvent> {
//     Command(Command<C>),
//     Event(ManagerEvent),
// }
//
// pub enum Command<T> {
//     Terminate,
//     Request(T),
// }
//
// // Send from From InnerStream task (eg/ WsStream) to SocketManager (event sourcing)
// // enum Message<Response, Forward, T> {
// //     Response(Response),
// //     ForwardToSink(Forward),
// //     Data(T),
// // }
// pub enum Message<ManagerEvent, StreamEvent> {
//     Manager(ManagerEvent),
//     Stream(StreamEvent),
// }
//
pub trait SocketParser<Output> {
    type Input;
    fn parse(input: Self::Input) -> impl Iterator<Item = Output>;
}

pub struct WebSocketParser;

impl SocketParser for WebSocketParser {
    type Input = Result<WsMessage, WsError>;

    fn parse<Output>(input: Self::Input) -> impl Iterator<Item = Output> {
        std::iter::empty()
    }
}

pub trait Transformer {
    type Input;
    type Output;
    fn transform(input: Self::Input) -> impl IntoIterator<Item = Self::Output>;
}

pub fn process<Parser, Transform, ManagerEvent, StreamEvent, FnRoute>(
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

pub fn parse_protocol_stream_and_transform<Parser, Transform>(
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
