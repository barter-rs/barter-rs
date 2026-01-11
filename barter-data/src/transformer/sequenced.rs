use crate::{
    Identifier,
    error::DataError,
    event::{MarketEvent, MarketIter},
    subscription::{Map, book::{OrderBookEvent, OrderBooksL2}},
    transformer::ExchangeTransformer,
};
use crate::exchange::Connector;
use barter_instrument::exchange::ExchangeId;
use barter_integration::{Transformer, protocol::websocket::WsMessage, subscription::SubscriptionId};
use std::{future::Future, marker::PhantomData};
use tokio::sync::mpsc;

/// Trait defining the logic for sequencing OrderBook L2 updates.
pub trait OrderBookL2Sequencer: Send + Sized {
    type Update: Identifier<Option<SubscriptionId>>;

    /// Construct a new [`OrderBookL2Sequencer`] from an optional initial [`OrderBookEvent`] snapshot.
    fn new(snapshot: Option<&OrderBookEvent>, sub_id: SubscriptionId) -> Result<Self, DataError>;

    /// Validate the sequence of the update.
    fn validate(&mut self, update: Self::Update) -> Result<Option<Self::Update>, DataError>;
}

#[derive(Debug)]
pub struct SequencedOrderBookL2Transformer<Exchange, InstrumentKey, Sequencer> {
    exchange_id: ExchangeId,
    instrument_map: Map<SequencedInstrument<InstrumentKey, Sequencer>>,
    phantom: PhantomData<Exchange>,
}

#[derive(Debug)]
pub struct SequencedInstrument<InstrumentKey, Sequencer> {
    pub key: InstrumentKey,
    pub sequencer: Sequencer,
}

impl<Exchange, InstrumentKey, Sequencer> ExchangeTransformer<Exchange, InstrumentKey, OrderBooksL2>
    for SequencedOrderBookL2Transformer<Exchange, InstrumentKey, Sequencer>
where
    Exchange: Connector,
    InstrumentKey: Clone + PartialEq + Send + Sync,
    Sequencer: OrderBookL2Sequencer,
    MarketIter<InstrumentKey, OrderBookEvent>: From<(ExchangeId, InstrumentKey, Sequencer::Update)>,
{
    fn init(
        instrument_map: Map<InstrumentKey>,
        initial_snapshots: &[MarketEvent<InstrumentKey, OrderBookEvent>],
        _: mpsc::UnboundedSender<WsMessage>,
    ) -> impl Future<Output = Result<Self, DataError>> + Send {
        async move {
            let instrument_map = instrument_map
                .0
                .into_iter()
                .map(|(sub_id, instrument_key)| {
                    // Find initial snapshot if it exists
                    let snapshot_event = initial_snapshots
                        .iter()
                        .find(|event| event.instrument == instrument_key)
                        .map(|event| &event.kind);

                    // If snapshot found, ensure it's a Snapshot event (not Update)
                    let snapshot = match snapshot_event {
                        Some(OrderBookEvent::Snapshot(_)) => snapshot_event,
                        Some(OrderBookEvent::Update(_)) => return Err(DataError::InitialSnapshotInvalid(
                            "expected OrderBookEvent::Snapshot but found OrderBookEvent::Update".to_string(),
                        )),
                        None => None,
                    };
                    
                    let sequencer = Sequencer::new(snapshot, sub_id.clone())?;
                    
                    Ok((sub_id, SequencedInstrument { key: instrument_key, sequencer }))
                })
                .collect::<Result<Map<_>, _>>()?;

            Ok(Self {
                exchange_id: Exchange::ID,
                instrument_map,
                phantom: PhantomData,
            })
        }
    }
}

impl<Exchange, InstrumentKey, Sequencer> Transformer for SequencedOrderBookL2Transformer<Exchange, InstrumentKey, Sequencer>
where
    Exchange: Connector, 
    InstrumentKey: Clone,
    Sequencer: OrderBookL2Sequencer,
    MarketIter<InstrumentKey, OrderBookEvent>: From<(ExchangeId, InstrumentKey, Sequencer::Update)>,
{
    type Error = DataError;
    type Input = Sequencer::Update;
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

        // Validate sequence
        let valid_update = match instrument.sequencer.validate(input) {
            Ok(Some(valid_update)) => valid_update,
            Ok(None) => return vec![],
            Err(error) => return vec![Err(error)],
        };

        MarketIter::<InstrumentKey, OrderBookEvent>::from((
            self.exchange_id,
            instrument.key.clone(),
            valid_update,
        ))
        .0
    }
}
