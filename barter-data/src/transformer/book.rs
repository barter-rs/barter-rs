use crate::{
    error::DataError,
    event::{MarketEvent, MarketIter},
    exchange::Connector,
    subscription::{book::OrderBook, Map, SubscriptionKind},
    transformer::ExchangeTransformer,
    Identifier,
};
use async_trait::async_trait;
use barter_integration::{
    model::{instrument::Instrument, SubscriptionId},
    protocol::websocket::WsMessage,
    Transformer,
};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use tokio::sync::mpsc;

/// Defines how to apply a [`Self::Update`] to an [`Self::OrderBook`].
#[async_trait]
pub trait OrderBookUpdater
where
    Self: Sized,
{
    type OrderBook;
    type Update;

    /// Initialises the [`InstrumentOrderBook`] for the provided [`Instrument`]. This often requires
    /// a HTTP call to receive a starting [`OrderBook`] snapshot.
    async fn init<Exchange, Kind>(
        ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
        instrument: Instrument,
    ) -> Result<InstrumentOrderBook<Instrument, Self>, DataError>
    where
        Exchange: Send,
        Kind: Send;

    /// Apply the [`Self::Update`] to the provided mutable [`Self::OrderBook`].
    fn update(
        &mut self,
        book: &mut Self::OrderBook,
        update: Self::Update,
    ) -> Result<Option<Self::OrderBook>, DataError>;
}

/// [`OrderBook`] for an [`Instrument`] with an exchange specific [`OrderBookUpdater`] to define
/// how to update it.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct InstrumentOrderBook<InstrumentId, Updater> {
    pub instrument: InstrumentId,
    pub updater: Updater,
    pub book: OrderBook,
}

/// Standard generic [`ExchangeTransformer`] to translate exchange specific OrderBook types into
/// normalised Barter OrderBook types. Requires an exchange specific [`OrderBookUpdater`]
/// implementation.
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct MultiBookTransformer<Exchange, InstrumentId, Kind, Updater> {
    pub book_map: Map<InstrumentOrderBook<InstrumentId, Updater>>,
    phantom: PhantomData<(Exchange, Kind)>,
}

#[async_trait]
impl<Exchange, Kind, Updater> ExchangeTransformer<Exchange, Instrument, Kind>
    for MultiBookTransformer<Exchange, Instrument, Kind, Updater>
where
    Exchange: Connector + Send,
    Kind: SubscriptionKind<Event = OrderBook> + Send,
    Updater: OrderBookUpdater<OrderBook = Kind::Event> + Send,
    Updater::Update: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
{
    async fn new(
        ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
        map: Map<Instrument>,
    ) -> Result<Self, DataError> {
        // Initialise InstrumentOrderBooks for all Subscriptions
        let (sub_ids, init_book_requests): (Vec<_>, Vec<_>) = map
            .0
            .into_iter()
            .map(|(sub_id, instrument)| {
                (
                    sub_id,
                    Updater::init::<Exchange, Kind>(ws_sink_tx.clone(), instrument),
                )
            })
            .unzip();

        // Await all initial OrderBook snapshot requests
        let init_order_books = futures::future::join_all(init_book_requests)
            .await
            .into_iter()
            .collect::<Result<Vec<InstrumentOrderBook<Instrument, Updater>>, DataError>>()?;

        // Construct OrderBookMap if all requests successful
        let book_map = sub_ids
            .into_iter()
            .zip(init_order_books.into_iter())
            .collect::<Map<InstrumentOrderBook<Instrument, Updater>>>();

        Ok(Self {
            book_map,
            phantom: PhantomData,
        })
    }
}

impl<Exchange, InstrumentId, Kind, Updater> Transformer
    for MultiBookTransformer<Exchange, InstrumentId, Kind, Updater>
where
    Exchange: Connector,
    InstrumentId: Clone,
    Kind: SubscriptionKind<Event = OrderBook>,
    Updater: OrderBookUpdater<OrderBook = Kind::Event>,
    Updater::Update: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
{
    type Error = DataError;
    type Input = Updater::Update;
    type Output = MarketEvent<InstrumentId, Kind::Event>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;

    fn transform(&mut self, update: Self::Input) -> Self::OutputIter {
        // Determine if the update has an identifiable SubscriptionId
        let subscription_id = match update.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        // Retrieve the InstrumentOrderBook associated with this update (snapshot or delta)
        let book = match self.book_map.find_mut(&subscription_id) {
            Ok(book) => book,
            Err(unidentifiable) => return vec![Err(DataError::Socket(unidentifiable))],
        };

        // De-structure for ease
        let InstrumentOrderBook {
            instrument,
            book,
            updater,
        } = book;

        // Apply update (snapshot or delta) to OrderBook & generate Market<OrderBook> snapshot
        match updater.update(book, update) {
            Ok(Some(book)) => {
                MarketIter::<InstrumentId, OrderBook>::from((
                    Exchange::ID,
                    instrument.clone(),
                    book,
                ))
                .0
            }
            Ok(None) => vec![],
            Err(error) => vec![Err(error)],
        }
    }
}
