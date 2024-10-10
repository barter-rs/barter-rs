use std::fmt::Debug;
use futures::{Stream, StreamExt};
use tracing::warn;
use crate::books::map::OrderBookMap;
use crate::books::OrderBook;
use crate::event::MarketEvent;
use crate::subscription::book::OrderBookEvent;

pub struct OrderBookManager {

}

pub async fn manage_order_books<BookMap, St, InstrumentKey>(
    books: BookMap,
    mut stream: St,
)
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
            continue
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
                    asks
                } = update;


            }
        }
    }
}