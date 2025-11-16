use crate::{
    AsyncTransformer,
    channel::{Tx, mpsc_unbounded},
    error::SocketError,
    protocol::websocket::{WebSocket, WsError, WsMessage, connect},
    stream::merge::merge,
};
use futures::{Stream, StreamExt};

const URL: &str = "wss://streams.fast.onetrading.com";

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
        std::future::ready(ManagerOutputEvent)
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
    //  Update:
    //   If Manager is managing State/Stream (eg/ SubscriptionState), does init_socket need dynamic
    //   &State? Probably just needs static config, since Manager "orchestrates" stream output
    //   via Sink.send() & event sourcing ManagerInputEvents
    //   Probably is fine to take a regular closure Fn() -> ... where it is possible to capture
    //   static config.
    let manager = FooManager { state: FooState };

    // aka. Audit stream
    let manager_output_event_stream = rx_manager
        .into_stream()
        .scan(manager, |manager, update| Some(manager.process(update)));

    // Todo: New Problem: Manager needs access to Sink right? Which refreshes upon reconnection
    //  Maybe I need an additional level of tokio task with an eternal Tx and Rx

    // Todo: want to merge Audit stream with inner StreamEvents, for now must use channel
    // let external_stream = merge(
    //     manager_output_event_stream,
    //     _
    // );

    // Todo: from the Sink docs:
    //   The Sink::send_all combinator is of particular importance: you can use it to send an entire
    //   stream to a sink, which is the simplest way to ultimately consume a stream.

    let handle = tokio::spawn(async move {
        let tx_external_stream = tx_external_stream.clone();
        loop {
            // Initialise Stream<Item = Message<ManagerEvent, StreamEvent>
            let (sink, stream) = init_socket(init).await.unwrap();

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
