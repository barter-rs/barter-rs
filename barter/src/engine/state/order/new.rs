use std::collections::hash_map::Entry;
use std::pin::Pin;
use std::task::{Context, Poll};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use fnv::FnvHashMap;
use futures::Stream;
use rust_decimal::Decimal;
use barter_execution::balance::{AssetBalance, Balance};
use barter_execution::error::ApiError;
use barter_execution::indexer::AccountEventIndexer;
use barter_execution::order::{CancelInFlight, Cancelled, ClientOrderId, OpenInFlight, OrderId, OrderKind, TimeInForce};
use barter_instrument::asset::{AssetIndex, QuoteAsset};
use barter_instrument::asset::name::AssetNameExchange;
use barter_instrument::exchange::{ExchangeId, ExchangeIndex};
use barter_instrument::instrument::InstrumentIndex;
use barter_instrument::Keyed;
use barter_integration::collection::FnvIndexSet;
use barter_integration::protocol::websocket::WebSocket;
use barter_integration::snapshot::Snapshot;
use crate::engine::state::asset::AssetState;
use crate::engine::state::position::Position;
use crate::Timed;

#[derive(Debug, Clone)]
pub struct Order<State = OrderState> {
    pub key: OrderKey,
    pub kind: OrderKind,
    pub time_in_force: TimeInForce,
    pub price: Decimal,
    pub quantity: Decimal,
    pub state: State,
}

#[derive(Debug, Clone)]
pub struct OrderKey {
    cid: ClientOrderId,
    time_exchange: DateTime<Utc>,
}

// Primarily just to pass around the Keyed<OrderKey, OrderState>, time_exchange, OrderState, that needs to be passed around
#[derive(Debug, Clone)]
pub enum OrderState<AssetKey = AssetIndex, InstrumentKey = InstrumentIndex> {
    OpenInFlight(OpenInFlight),
    Open(Open),
    CancelInFlight(CancelInFlight),
    Cancelled(Cancelled),
    FullyFilled,
    Rejected(OrderError<AssetKey, InstrumentKey>),
    Expired,
}

impl<AssetKey, InstrumentKey> OrderState<AssetKey, InstrumentKey> {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::OpenInFlight(_) | Self::Open(_) | Self::CancelInFlight(_))
    }

}

#[derive(Debug, Clone)]
pub struct Open {
    pub id: OrderId,
    pub filled_quantity: Decimal,
}

#[derive(Debug, Clone)]
pub enum OrderError<AssetKey, InstrumentKey> {
    Connectivity,
    Api(ApiError<AssetKey, InstrumentKey>)
}


#[derive(Debug, Clone)]
pub struct Orders {
    orders: FnvHashMap<ClientOrderId, Order>,
    num_in_flight: u32,
}

impl Orders {
    // Handles OrderManager & InFlightRequestRecord
    pub fn update_from_snapshot(&mut self, snapshot: Snapshot<&Order>) {
        // Todo: add logging and ensure no cleanup operation is required with this paradigm

        let Snapshot(snapshot) = snapshot;

        let mut current = match self.orders.entry(snapshot.cid.clone()) {
            Entry::Vacant(entry) => {
                if snapshot.state.is_active() {
                    entry.insert(snapshot.clone());
                }
                return
            }
            Entry::Occupied(mut current) => current,
        };

        if current.get().key.time_exchange > snapshot.key.time_exchange {
            return;
        }

        if snapshot.state.is_active() {
            let current_mut = current.get_mut();
            current_mut.key.time_exchange = snapshot.key.time_exchange;
            current_mut.state = snapshot.state.clone();
        } else {
            current.remove();
        }
    }
}

// This would actually live downstream: ** THIS
// - Perhaps a StreamCombinator to create a Stream<State> via Fold / Scan
// - State could be State or State + TradingHistory
pub struct OrderAudit(Order<Vec<OrderState>>);

// Maybe it's time to re-try the `AccountSocket`, but in a re-connecting paradigm where
// if AccountStream DC's the internal Stream is restarted.

// Stream of AccountSocket
// Does it make sense to hold AccountState in the AccountSocket - is it needed?
//  Only if we want to associate Trades with StrategyIds before they are sent to the Engine
pub struct AccountSocket {
    // state: AccountState,
    indexer: AccountEventIndexer,
    socket: WebSocket,
    // What other data might we want here? Does it make sense to maintain ExchangePositions?

    // Todo: If Strategy is further abstracted so it provides eg:
    //  1. Output taker orders
    //  2. Desired open orders
    //  Then this AccountSocket would need to have a CidGenerator
    //  And Engine would handle RequestMeta such as RequestId
    //  Engine would have HashMap<Request, _> and also Orders<Cid>..?
    // cid_generator
}





pub struct AccountState {
    pub exchange: Keyed<ExchangeIndex, ExchangeId>,
    // pub assets: FnvIndexSet<Timed<Balance>>, // Could be a Vec since it's just snapshot
    // pub instruments: FnvIndexSet<InstrumentAccountState>
    pub assets: Vec<AssetBalance<AssetIndex>>,
    pub instruments: Vec<Keyed<InstrumentIndex, InstrumentAccountState>>

    // orders: FnvIndexSet<Orders>, // Add trade method to make it easy to associated Trade with Order
    // balances: FnvIndexSet<AssetBalance<AssetIndex>>,
}

pub struct InstrumentAccountState {
    pub position: Option<Position<QuoteAsset, InstrumentIndex>>,
    pub orders: Orders,
}

pub fn run<Rx, Tx>(request_rx: Rx, event_tx: Tx) {

    // Initialise Stream of AccountSocket (ie/ reconnecting Stream)
    //

}

// I want a Stream of (State + Event) or (State + History + Event)

// pub struct EngineStateAudit<State> {
//     pub state:
// }

// #[derive(Debug, Constructor)]
// pub struct StateStream<St, State> {
//     pub stream: St,
//     pub state: State,
// }
//
// impl<St, State> StateStream<St, State> {
//     pub fn add_audit
// }
//
// impl<St, State> Stream for StateStream<St, State>
// where
//     St: Stream,
// {
//     type Item = (State, St::Item);
//
//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//         todo!()
//     }
// }
//
// pub struct
//
//
//
// // Simplified AuditStream aggregation
// pub trait AuditStream
// where
//     Self: Stream,
// {
//     fn
//
// }
//
// fn aggregate() ->





