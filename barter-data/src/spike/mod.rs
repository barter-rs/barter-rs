use crate::subscription::Subscription;
use barter_instrument::{exchange::ExchangeId, instrument::kind::InstrumentKind};
use barter_integration::{
    channel::{UnboundedTx, mpsc_unbounded},
    protocol::websocket::{WebSocket, WsMessage, WsSink, connect},
    stream::merge::merge,
};
use fnv::FnvHashMap;
use futures::Stream;
use futures_util::{FutureExt, StreamExt};
use std::sync::Arc;

const URL: &str = "wss://streams.fast.onetrading.com";

pub type Initialised<Stream, Update> = (Stream, UnboundedTx<Update>);

// Open Q: Do I want to create a Sink implementation at the client level?
// It's Item would be eg/ BinanceRequest, not sure what the mirrored Stream impl would look like.

pub trait AsyncProcessor {
    type Event;
    type Audit;
    fn process(&mut self, event: Self::Event) -> impl Future<Output = Self::Audit>;
}

async fn implementation() {
    let websocket: WebSocket = connect(URL).await.unwrap();
    let (sink, stream) = websocket.split();

    let (tx_manager, rx_manager) = mpsc_unbounded();
    let manager_updates = rx_manager.into_stream();

    let manager = ConnectionManager {
        state: ConnectionState::new(
            PongState,
            [(ExchangeId::BinanceSpot, "btc", "usdt", InstrumentKind::Spot)],
        ),
        sink,
    };

    let stream = framework(manager, manager_updates, stream);
}

fn framework<Manager, Update>(
    manager: Manager,
    manager_updates: impl Stream<Item = Update>,
    stream: impl Stream<Item = WsMessage>,
) -> impl Stream<Item = SocketMessage<Manager::Event, Manager::Audit>>
where
    Manager: AsyncProcessor,
{
    let audits = manager_updates.scan(manager, |manager, update| Some(manager.process(update)));

    let stream = merge(
        stream.map(SocketMessage::Event),
        audits.map(SocketMessage::Audit),
    );

    Ok(stream)
}

pub struct ConnectionManager {
    state: ConnectionState,
    sink: WsSink,
}

// Todo: I think there should be two layers:
// OuterStream:
//  - Holds ConnectionState: if Pong issue it can re-init the InnerStream
// InnerStream:
//  - Holds no State. Must present WsSink upon re-init for OuterStream consistency.
impl AsyncProcessor for ConnectionManager {
    type Event = ConnectionUpdate;
    type Audit = ();

    async fn process(&mut self, event: Self::Event) -> Self::Audit {
        match event {
            ConnectionUpdate::Pong(_) => {}
            ConnectionUpdate::Subscription(_) => {}
        }

        todo!()
    }
}

pub struct ConnectionState {
    pong: PongState,
    subscriptions: FnvHashMap<Subscription, SubscriptionState>,
}

impl ConnectionState {
    pub fn new<SubIter, Sub>(pong: PongState, subscriptions: SubIter) -> Self
    where
        SubIter: IntoIterator<Item = Sub>,
        Sub: Into<Subscription>,
    {
        Self {
            pong,
            subscriptions: subscriptions.into_iter().map(Sub::into).collect(),
        }
    }
}

pub struct PongState;

pub enum SubscriptionState {
    SubscribeInFlight,
    Active,
    UnsubscribeInFlight,
}

pub enum ConnectionUpdate {
    Pong(u64),
    Subscription(Result<(), ()>),
}

pub enum SocketMessage<Event, Audit> {
    Event(Event),
    Audit(Audit),
}
