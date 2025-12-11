use barter_instrument::Keyed;
use barter_integration::{
    Admin, Message, Routing, Transformer,
    exchange::bitmex::websocket::{
        BITMEX_WS_BASE_URL, BitmexEvent, BitmexMessage, BitmexResponse, request::BitmexRequest,
    },
    protocol::websocket::{AdminWs, WsError, WsSinkError, init_websocket},
    serde::{de::DeJson, se::SeString},
    stream::ext::index::Indexer,
};
use fnv::FnvHashMap;
use futures::{
    Sink, SinkExt, Stream, StreamExt,
    future::Either::{Left, Right},
    stream,
};

pub enum IndexUpdate {
    Add,
    Remove,
    Modify,
}

async fn init_websocket_bitmex() -> Result<
    impl Sink<BitmexRequest, Error = WsSinkError>
    + Stream<Item = Message<AdminWs, BitmexMessage>>
    + Send,
    WsError,
> {
    init_websocket::<SeString, BitmexRequest, String, DeJson<BitmexMessage>, BitmexMessage>(
        BITMEX_WS_BASE_URL,
    )
    .await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    run().await
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut socket = init_websocket_bitmex().await?;

    // Todo: At this point we need to Index (ie/ InstrumentKey) and also determine Routing
    // let socket = socket.with_indexer(MessageIndexer::default());

    let request = BitmexRequest::subscribe(["trade:XBTUSD"]);
    socket.send(request).await.unwrap();

    let mut socket =
        apply_route::<BitmexRequest, BitmexMessage, BitmexForwardResponses, _, _>(socket);

    while let Some(message) = socket.next().await {
        println!("{message:?}");
    }

    Ok(())
}

#[derive(Debug)]
pub struct MessageIndexer<Key, Index> {
    pub map: FnvHashMap<Key, Index>,
}

impl<Key, Index> Default for MessageIndexer<Key, Index> {
    fn default() -> Self {
        Self {
            map: <_>::default(),
        }
    }
}

impl<Key, Index> Indexer for MessageIndexer<Key, Index> {
    type Unindexed = Message<AdminWs, BitmexMessage>;
    type Indexed = Message<AdminWs, Keyed<Index, BitmexMessage>>;
    type Error = ();

    fn index(&self, item: Self::Unindexed) -> Result<Self::Indexed, Self::Error> {
        todo!()
    }
}

pub struct BitmexForwardResponses;

impl Transformer<BitmexMessage> for BitmexForwardResponses {
    type Output<'a> = Routing<BitmexResponse, BitmexEvent>;

    fn transform<'a>(
        input: BitmexMessage,
    ) -> impl IntoIterator<Item = Self::Output<'a>, IntoIter: Send> {
        std::iter::once(match input {
            BitmexMessage::Response(response) => Routing::Forward(response),
            BitmexMessage::Event(event) => Routing::Keep(event),
        })
    }
}

pub fn apply_route<Out, In, Router, Forward, Keep>(
    socket: impl Sink<Out> + Stream<Item = Message<AdminWs, In>>,
) -> impl Sink<Out> + Stream<Item = Message<Admin<AdminWs, Forward>, Keep>>
where
    for<'a> In: 'a,
    Router: for<'a> Transformer<In, Output<'a> = Routing<Forward, Keep>>,
{
    use futures::{StreamExt, future::Either::*, stream};
    use itertools::Either::{Left as IterLeft, Right as IterRight};

    socket.flat_map(|message| match message {
        Message::Admin(admin) => Left(stream::iter(std::iter::once(Message::Admin(
            Admin::Protocol(admin),
        )))),
        Message::Payload(payload) => Right(stream::iter(
            Router::transform(payload)
                .into_iter()
                .flat_map(|routing| match routing {
                    Routing::Forward(forward) => {
                        IterLeft(std::iter::once(Message::Admin(Admin::Application(forward))))
                    }
                    Routing::Keep(keep) => IterLeft(std::iter::once(Message::Payload(keep))),
                    Routing::ForwardAndKeep { forward, keep } => IterRight(
                        [
                            Message::Admin(Admin::Application(forward)),
                            Message::Payload(keep),
                        ]
                        .into_iter(),
                    ),
                }),
        )),
    })
}

pub fn init_logging() {
    use tracing_subscriber::{
        prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .with(tracing_subscriber::fmt::layer())
        .init()
}
