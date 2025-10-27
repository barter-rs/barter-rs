use crate::{
    Identifier,
    books::{
        OrderBook,
        map::{IndexedOrderBookMapMulti, OrderBookMap, OrderBookMapMulti},
    },
    error::DataError,
    exchange::{
        StreamSelector,
        binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
    },
    instrument::{InstrumentData, MarketInstrumentData},
    streams::{
        Streams, builder::StreamBuilder, consumer::MarketStreamEvent,
        reconnect::stream::ReconnectingStream,
    },
    subscription::{
        Subscription, SubscriptionKind,
        book::{OrderBookEvent, OrderBooksL2},
    },
};
use barter_instrument::{
    Keyed,
    exchange::ExchangeId,
    index::IndexedInstruments,
    instrument::{InstrumentIndex, name::InstrumentNameInternal},
};
use barter_integration::collection::FnvIndexMap;
use fnv::FnvHashMap;
use futures::Stream;
use futures_util::StreamExt;
use itertools::Itertools;
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
    St: Stream<Item = MarketStreamEvent<BookMap::LookupKey, OrderBookEvent>> + Unpin,
    BookMap: OrderBookMap,
    BookMap::LookupKey: Debug,
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
        impl OrderBookMap<LookupKey = Instrument::Key>,
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
    let stream = init_stream(stream_builder).await?;

    Ok(OrderBookL2Manager {
        stream,
        books: OrderBookMapMulti::new(books),
    })
}

/// Initializes an [OrderBookL2Manager] from the provided collection of [IndexedInstruments].
/// See `examples/indexed_order_books_l2_manager` for an example of how to initialize and access books when
///  using this initialization paradigm.
/// # Arguments
/// * `instruments` - Reference to [`IndexedInstruments`] containing what instruments are being used.
//TODO -  Tried to use a reference to IndexedInstruments, but couldn't resolve the lifetime issues.
pub async fn init_indexed_multi_order_book_l2_manager(
    instruments: IndexedInstruments,
) -> Result<
    OrderBookL2Manager<
        impl Stream<Item = MarketStreamEvent<InstrumentIndex, OrderBookEvent>>,
        IndexedOrderBookMapMulti<InstrumentNameInternal>,
    >,
    DataError,
> {
    // Generate Iterator<Item = MarketInstrumentData<InstrumentKey>>
    let instruments = instruments.instruments().iter().map(|keyed| {
        let exchange = keyed.value.exchange.value;
        let instrument = MarketInstrumentData::from(keyed);
        Keyed::new(exchange, instrument)
    });

    let (stream_builder, books) = instruments
        .chunk_by(|exchange| exchange.key)
        .into_iter()
        .fold(
            (Streams::<OrderBooksL2>::builder(), FnvIndexMap::default()),
            |(builder, mut books), (exchange, instruments)| {
                if exchange == ExchangeId::BinanceSpot {
                    let batch = instruments.into_iter().map(
                        |Keyed {
                             key: _exchange,
                             value: instrument,
                         }| {
                            {
                                books.insert(
                                    instrument.name_internal.clone(),
                                    Arc::new(RwLock::new(OrderBook::default())),
                                );

                                create_subscription::<BinanceSpot>(instrument)
                            }
                        },
                    );

                    let builder = builder.subscribe(batch);

                    (builder, books)
                } else if exchange == ExchangeId::BinanceFuturesUsd {
                    let batch = instruments.into_iter().map(
                        |Keyed {
                             key: _exchange,
                             value: instrument,
                         }| {
                            {
                                books.insert(
                                    instrument.name_internal.clone(),
                                    Arc::new(RwLock::new(OrderBook::default())),
                                );

                                create_subscription::<BinanceFuturesUsd>(instrument)
                            }
                        },
                    );

                    let builder = builder.subscribe(batch);

                    (builder, books)
                } else {
                    panic!("Unexpected exchange: {:?}", exchange);
                }
            },
        );

    // Initialise merged OrderBookL2 Stream
    let stream = init_stream(stream_builder).await?;

    Ok(OrderBookL2Manager {
        stream,
        books: IndexedOrderBookMapMulti::new(books),
    })
}

/// Creates a [`Subscription`] for the given [`MarketInstrumentData`], using the provided [`Exchange`] type and [`OrderBooksL2`] kind.
///
/// # Type Parameters
/// - `E`: The Exchange type to associate with the [`Subscription`]. Must implement [`Default`].
///
/// # Arguments
/// - `instrument`: The [`MarketInstrumentData`] used to construct the [`Subscription`].
///
/// # Returns
/// A new [`Subscription`] instance configured with the given instrument, the exchange's default value, and [`OrderBooksL2`] as the subscription kind.
fn create_subscription<E>(
    instrument: MarketInstrumentData<InstrumentIndex>,
) -> Subscription<E, MarketInstrumentData<InstrumentIndex>, OrderBooksL2>
where
    E: Default,
{
    Subscription::new(E::default(), instrument, OrderBooksL2)
}

/// Initializes a [`Stream`] from the provided [`StreamBuilder`] and applies error handling.
///
/// This function asynchronously builds the underlying stream using [`StreamBuilder::init`],
/// merges multiple substreams into a single stream with [`select_all`], and attaches a
/// recoverable error handler that logs warnings for non-fatal stream errors.
///
/// # Type Parameters
/// - `K`: The key type used to identify instruments in the stream.
/// - `Kind`: The subscription kind associated with the stream.
///
pub async fn init_stream<K, Kind>(
    stream_builder: StreamBuilder<K, Kind>,
) -> Result<impl Stream<Item = MarketStreamEvent<K, OrderBookEvent>>, DataError>
where
    K: Eq + Hash + Send + Debug + 'static,
    Kind: SubscriptionKind<Event = OrderBookEvent> + Send + 'static,
{
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

    Ok(stream)
}
