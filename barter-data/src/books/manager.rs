use crate::books::map::OrderBookMap;
use crate::books::OrderBook;
use crate::event::MarketEvent;
use crate::subscription::book::OrderBookEvent;
use barter_integration::model::Exchange;
use futures::{Stream, StreamExt};
use parking_lot::RwLock;
use std::fmt::Debug;
use std::sync::Arc;
use tracing::warn;

// Todo: Open Questions:
//  - How can each SingleBookManager "own" the input Stream, so it can restart it etc.
//  - Or Perhaps one BookManager per Exchange/Connection?
//     OR can have a SnapshotTransformer that fetches an initial snapshot from the exchange...
//          '--> leaning towards this

// pub struct MultiBookManager<InstrumentKey> {
//     pub books: FnvHashMap<InstrumentKey, SingleBookManager>,
// }
//
// impl<InstrumentKey> MultiBookManager<InstrumentKey> {
//     pub fn run<St>(self, mut stream: St)
//     where
//         St: Stream<Item = MarketEvent<InstrumentKey, OrderBookEvent>>
//     {
//
//     }
// }

pub struct SingleBookManager<FnSnapshot> {
    pub exchange: Exchange,
    pub fetch_snapshot: FnSnapshot,
    pub book: Arc<RwLock<OrderBook>>,
}


pub async fn manage_order_books<BookMap, St, InstrumentKey>(books: BookMap, mut stream: St)
where
    BookMap: OrderBookMap<Key = InstrumentKey>,
    St: Stream<Item = MarketEvent<InstrumentKey, OrderBookEvent>> + Unpin,
    InstrumentKey: Debug,
{
    while let Some(event) = stream.next().await {
        let Some(book) = books.find(&event.instrument) else {
            warn!(
                instrument = ?event.instrument,
                "consumed MarketEvent<OrderBookEvent> for non-configured instrument"
            );
            continue;
        };

        // Todo: consider OrderBook<Kind> for Snapshot & Update { meta }

        match event.kind {
            OrderBookEvent::Snapshot(snapshot) => {
                *book.write() = snapshot;
            }
            OrderBookEvent::Update(update) => {
                let OrderBook {
                    sequence,
                    time_engine,
                    bids,
                    asks,
                } = update;
            }
        }
    }
}
