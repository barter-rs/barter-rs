use crate::{
    channel::{Tx, mpsc_unbounded},
    error::SocketError,
    protocol::websocket::{WebSocket, WsError, WsMessage, connect},
    socket::{Message, SocketParser, Transformer, route_manager_events},
    stream::merge::merge,
};
use futures::{Stream, StreamExt};

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
struct ManagerInputEvent(String);
#[derive(Clone, Debug)]
struct StreamEvent(String);
struct ManagerOutputEvent; // aka Audit
struct FooManager {
    state: FooState,
}
impl AsyncTransformer for FooManager {
    type Input = ManagerInputEvent;
    type Output = ManagerOutputEvent;
    fn transform(&mut self, input: Self::Input) -> impl Future<Output = Self::Output> {
        todo!()
    }
}
struct Output<Stream> {
    external_stream: Stream,
    handle: tokio::task::JoinHandle<()>,
}

async fn run() {
    // Init contains all Stream processing
    let init = |state: &FooState| async {
        let websocket: WebSocket = connect(URL).await?;
        let (sink, stream) = websocket.split();

        // Todo: SocketParser + Transformer using parse_protocol_stream_and_transform
        let stream = stream.map(|ws_result: Result<WsMessage, WsError>| match ws_result {
            Ok(message) => Message::Stream(StreamEvent(message.to_string())),
            Err(error) => Message::Manager(ManagerInputEvent(error.to_string())),
        });

        Ok::<_, SocketError>((sink, stream))
    };

    let (tx_manager, rx_manager) = mpsc_unbounded::<ManagerInputEvent>();
    let route = |event: ManagerInputEvent| async { tx_manager.send(event).map_err(|_| ()) };

    let (tx_external_stream, rx_external_stream) =
        mpsc_unbounded::<SocketMessage<StreamEvent, ManagerOutputEvent>>();
    let external_stream = rx_external_stream.into_stream();

    // Todo: Problem: Manager has to process(ManagerInputEvents) as well as init_socket()
    let manager = FooManager { state: FooState };
    let manager_output_event_stream = rx_manager
        .into_stream()
        .scan(manager, |manager, update| Some(manager.process(update)));

    let handle = tokio::spawn(async move {
        let tx_external_stream = tx_external_stream.clone();
        loop {
            // Initialise Stream<Item = Message<ManagerEvent, StreamEvent>
            let (sink, stream) = init_socket(init, &manager.state).await.unwrap();

            // ManagerEvents routed, so left with StreamEvents to send to external stream
            let stream = route_manager_events::<ManagerInputEvent, StreamEvent, _>(stream, route);
            let mut stream = std::pin::pin!(stream);

            // Forward StreamEvents to external Stream
            while let Some(event) = stream.next().await {
                tx_external_stream.send(event).unwrap();
            }

            // todo: Below is great but need access to State since it's used to init_socket (see above todo)
            //     let audits = manager_updates
            //         .scan(manager, |manager, update| Some(manager.process(update)));
            //
            //     let stream = merge(
            //         stream.map(SocketMessage::Event),
            //         audits.map(SocketMessage::Audit)
            //     );
        }
    });
}

// Do I want this to be a Stream of Streams? Because State is changing I don't think so.
// Unless Arc<Mutex<>>... surely not.
pub async fn init_socket<FnInit, FnInitErr, State, Sink, Stream>(
    mut init: FnInit,
    state: &State,
) -> Result<(Sink, Stream), FnInitErr>
where
    FnInit: AsyncFnMut(&State) -> Result<(Sink, Stream), FnInitErr>,
{
    let (sink, stream) = init(state).await?;
    Ok((sink, stream))
}

pub fn route_manager_events<ManagerEvent, StreamEvent, FnRoute>(
    stream: impl Stream<Item = Message<ManagerInputEvent, StreamEvent>> + Unpin + 'static,
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

// Todo: SocketMessage and Manager::Input & Manager::Output here are confusing because there is also
//       the ManagerEvent & StreamEvent generics
//      --> ManagerEvent should be SendToManagerEvent,
//      --> StreamEvent should be RawStreamEvent,
//      --> Not sure how SocketMessage relates - do I want to combine all into the external stream?
fn framework<Manager, Update>(
    manager: Manager,
    manager_updates: impl Stream<Item = Update>,
    stream: impl Stream<Item = WsMessage>,
) -> impl Stream<Item = SocketMessage<Manager::Input, Manager::Output>>
where
    Manager: AsyncTransformer,
{
    let audits = manager_updates.scan(manager, |manager, update| Some(manager.process(update)));

    let stream = merge(
        stream.map(SocketMessage::Event),
        audits.map(SocketMessage::Audit),
    );

    Ok(stream)
}

pub enum SocketMessage<Event, Audit> {
    Event(Event),
    Audit(Audit),
}
