use crate::{
    Identifier,
    books::{
        OrderBook,
        map::{OrderBookMap, OrderBookMapMulti},
    },
    error::DataError,
    exchange::StreamSelector,
    instrument::InstrumentData,
    streams::{Streams, consumer::MarketStreamEvent, reconnect::stream::ReconnectingStream},
    subscription::{
        Subscription,
        book::{OrderBookEvent, OrderBooksL2},
    },
};
use fnv::FnvHashMap;
use futures::Stream;
use futures_util::StreamExt;
use parking_lot::RwLock;
use std::{
    fmt::{Debug, Display},
    hash::Hash,
    sync::Arc,
};
use tracing::warn;

/// Maintains a set of local L2 [`OrderBook`]s by applying streamed [`OrderBookEvent`]s to the
/// associated [`OrderBook`] in the [`OrderBookMap`].
#[derive(Debug)]
pub struct OrderBookL2Manager<St, BookMap> {
    pub stream: St,
    pub books: BookMap,
}

impl<St, BookMap> OrderBookL2Manager<St, BookMap>
where
    St: Stream<Item = MarketStreamEvent<BookMap::Key, OrderBookEvent>> + Unpin,
    BookMap: OrderBookMap,
    BookMap::Key: Debug,
{
    /// Manage local L2 [`OrderBook`]s.
    pub async fn run(mut self) {
        while let Some(stream_event) = self.stream.next().await {
            // Extract MarketEvent<InstrumentKey, OrderBookEvent>
            let event = match stream_event {
                MarketStreamEvent::Reconnecting(exchange) => {
                    warn!(%exchange, "OrderBook manager input stream disconnected");
                    continue;
                }
                MarketStreamEvent::Item(event) => event,
            };

            // Find OrderBook associated with the MarketEvent InstrumentKey
            let Some(book) = self.books.find(&event.instrument) else {
                warn!(
                    instrument = ?event.instrument,
                    "consumed MarketStreamEvent<_, OrderBookEvent> for non-configured instrument"
                );
                continue;
            };

            let mut book_lock = book.write();
            book_lock.update(&event.kind);
        }
    }
}

/// Initialise a [`OrderBookL2Manager`] using the provided batches of [`OrderBooksL2`]
/// [`Subscription`]s.
///
/// See `examples/order_books_l2_manager` for how to use this initialisation paradigm.
pub async fn init_multi_order_book_l2_manager<SubBatchIter, SubIter, Sub, Exchange, Instrument>(
    subscription_batches: SubBatchIter,
) -> Result<
    OrderBookL2Manager<
        impl Stream<Item = MarketStreamEvent<Instrument::Key, OrderBookEvent>>,
        impl OrderBookMap<Key = Instrument::Key>,
    >,
    DataError,
>
where
    SubBatchIter: IntoIterator<Item = SubIter>,
    SubIter: IntoIterator<Item = Sub>,
    Sub: Into<Subscription<Exchange, Instrument, OrderBooksL2>>,
    Exchange: StreamSelector<Instrument, OrderBooksL2> + Ord + Display + Send + Sync + 'static,
    Instrument: InstrumentData + Ord + Display + 'static,
    Instrument::Key: Eq + Hash + Send + 'static,
    Subscription<Exchange, Instrument, OrderBooksL2>:
        Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
{
    // Generate Streams from provided OrderBooksL2 Subscription batches
    let (stream_builder, books) = subscription_batches.into_iter().fold(
        (Streams::<OrderBooksL2>::builder(), FnvHashMap::default()),
        |(builder, mut books), batch| {
            // Insert OrderBook Entry for each unique Subscription (duplicates upserted)
            let batch = batch.into_iter().map(|sub| {
                let subscription = sub.into();
                books.insert(
                    subscription.instrument.key().clone(),
                    Arc::new(RwLock::new(OrderBook::default())),
                );
                subscription
            });

            let builder = builder.subscribe(batch);
            (builder, books)
        },
    );

    // Initialise merged OrderBookL2 Stream
    let stream = stream_builder
        .init()
        .await?
        .select_all()
        .with_error_handler(|error| {
            warn!(
                ?error,
                "OrderBookL2Manager consumed recoverable MarketStream error"
            )
        });

    Ok(OrderBookL2Manager {
        stream,
        books: OrderBookMapMulti::new(books),
    })
}
