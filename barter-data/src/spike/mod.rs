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

    // let stream = framework(manager, manager_updates, stream);
}

pub struct ConnectionManager {
    state: ConnectionState,
    sink: WsSink,
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
