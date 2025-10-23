use async_trait::async_trait;
use barter_instrument::exchange::ExchangeId;
use barter_integration::{Transformer, error::SocketError, protocol::websocket::WsMessage};
use futures::future::try_join_all;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    Identifier, SnapshotFetcher,
    error::DataError,
    event::{MarketEvent, MarketIter},
    exchange::bitstamp::{
        BitstampSpot,
        book::{
            BitstampOrderBookL2Meta,
            message::{BitstampOrderBookL2Inner, BitstampOrderBookL2Message},
        },
        market::BitstampMarket,
    },
    instrument::InstrumentData,
    subscription::{
        Map, Subscription,
        book::{OrderBookEvent, OrderBooksL2},
    },
    transformer::ExchangeTransformer,
};

pub const HTTP_BOOK_L2_SNAPSHOT_URL_BITSTAMP: &str = "https://www.bitstamp.net/api/v2/order_book";

#[derive(Debug)]
pub struct BitstampOrderBooksL2SnapshotFetcher;

impl SnapshotFetcher<BitstampSpot, OrderBooksL2> for BitstampOrderBooksL2SnapshotFetcher {
    fn fetch_snapshots<Instrument>(
        subscriptions: &[Subscription<BitstampSpot, Instrument, OrderBooksL2>],
    ) -> impl Future<Output = Result<Vec<MarketEvent<Instrument::Key, OrderBookEvent>>, SocketError>>
    + Send
    where
        Instrument: InstrumentData,
        Subscription<BitstampSpot, Instrument, OrderBooksL2>: Identifier<BitstampMarket>,
    {
        let l2_snapshot_futures = subscriptions.iter().map(|sub| {
            // Construct initial OrderBook snapshot GET url
            let market = sub.id();
            let snapshot_url =
                format!("{}/{}", HTTP_BOOK_L2_SNAPSHOT_URL_BITSTAMP, market.as_ref(),);

            async move {
                // Fetch initial OrderBook snapshot via HTTP
                let snapshot = reqwest::get(snapshot_url)
                    .await
                    .map_err(SocketError::Http)?
                    .json::<BitstampOrderBookL2Inner>()
                    .await
                    .map_err(SocketError::Http)?;

                Ok(MarketEvent::from((
                    ExchangeId::Bitstamp,
                    sub.instrument.key().clone(),
                    snapshot,
                )))
            }
        });

        try_join_all(l2_snapshot_futures)
    }
}

#[derive(Debug)]
pub struct BitstampOrderBooksL2Transformer<InstrumentKey> {
    instrument_map: Map<BitstampOrderBookL2Meta<InstrumentKey, BitstampOrderBookL2Sequencer>>,
}

#[async_trait]
impl<InstrumentKey> ExchangeTransformer<BitstampSpot, InstrumentKey, OrderBooksL2>
    for BitstampOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone + PartialEq + Send + Sync,
{
    async fn init(
        instrument_map: Map<InstrumentKey>,
        initial_snapshots: &[MarketEvent<InstrumentKey, OrderBookEvent>],
        _: UnboundedSender<WsMessage>,
    ) -> Result<Self, DataError> {
        let instrument_map = instrument_map
            .0
            .into_iter()
            .map(|(sub_id, instrument_key)| {
                let snapshot = initial_snapshots
                    .iter()
                    .find(|snapshot| snapshot.instrument == instrument_key)
                    .ok_or_else(|| DataError::InitialSnapshotMissing(sub_id.clone()))?;

                let OrderBookEvent::Snapshot(snapshot) = &snapshot.kind else {
                    return Err(DataError::InitialSnapshotInvalid(String::from(
                        "expected OrderBookEvent::Snapshot but found OrderBookEvent::Update",
                    )));
                };

                let sequencer = BitstampOrderBookL2Sequencer::new(snapshot.sequence());

                Ok((
                    sub_id,
                    BitstampOrderBookL2Meta::new(instrument_key, sequencer),
                ))
            })
            .collect::<Result<Map<_>, _>>()?;

        Ok(Self { instrument_map })
    }
}

impl<InstrumentKey> Transformer for BitstampOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone,
{
    type Error = DataError;
    type Input = BitstampOrderBookL2Message;
    type Output = MarketEvent<InstrumentKey, OrderBookEvent>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        // Determine if the message has an identifiable SubscriptionId
        let subscription_id = match input.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        // Find Instrument associated with Input and transform
        let instrument = match self.instrument_map.find_mut(&subscription_id) {
            Ok(instrument) => instrument,
            Err(unidentifiable) => return vec![Err(DataError::from(unidentifiable))],
        };

        // Drop any outdated updates & validate sequence for relevant updates
        let valid_update = match instrument.sequencer.validate_sequence(input) {
            Ok(Some(valid_update)) => valid_update,
            Ok(None) => return vec![],
            Err(error) => return vec![Err(error)],
        };

        MarketIter::<InstrumentKey, OrderBookEvent>::from((
            ExchangeId::Bitstamp,
            instrument.key.clone(),
            valid_update,
        ))
        .0
    }
}

#[derive(Debug)]
pub struct BitstampOrderBookL2Sequencer {
    last_microtimestamp: u64,
}

impl BitstampOrderBookL2Sequencer {
    pub fn new(last_microtimestamp: u64) -> Self {
        Self {
            last_microtimestamp,
        }
    }

    pub fn validate_sequence(
        &mut self,
        update: BitstampOrderBookL2Message,
    ) -> Result<Option<BitstampOrderBookL2Message>, DataError> {
        // TODO: This implementation is probably not correct. Not sure if we can
        // make it better.
        if update.data.timestamp <= self.last_microtimestamp {
            return Ok(None);
        } else {
            Ok(Some(update))
        }
    }
}
