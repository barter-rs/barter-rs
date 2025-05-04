use std::vec;

use crate::{
    Identifier,
    error::DataError,
    event::{MarketEvent, MarketIter},
    exchange::{
        Connector,
        bybit::{Bybit, message::BybitPayloadKind, spot::BybitSpot},
    },
    subscription::{
        Map,
        book::{OrderBookEvent, OrderBooksL2},
    },
    transformer::ExchangeTransformer,
};
use async_trait::async_trait;
use barter_integration::{Transformer, protocol::websocket::WsMessage};
use derive_more::Constructor;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

use super::BybitOrderBookMessage;

#[derive(Debug, Constructor)]
pub struct BybitOrderBookL2Meta<InstrumentKey, Sequencer> {
    pub key: InstrumentKey,
    pub sequencer: Option<Sequencer>,
}

#[derive(Debug)]
pub struct BybitOrderBooksL2Transformer<InstrumentKey> {
    instrument_map: Map<BybitOrderBookL2Meta<InstrumentKey, BybitOrderBookL2Sequencer>>,
}

#[async_trait]
impl<InstrumentKey, Server> ExchangeTransformer<Bybit<Server>, InstrumentKey, OrderBooksL2>
    for BybitOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone + PartialEq + Send + Sync,
{
    async fn init(
        instrument_map: Map<InstrumentKey>,
        _: &[MarketEvent<InstrumentKey, OrderBookEvent>],
        _: UnboundedSender<WsMessage>,
    ) -> Result<Self, DataError> {
        let instrument_map = instrument_map
            .0
            .into_iter()
            .map(|(sub_id, instrument_key)| {
                (sub_id, BybitOrderBookL2Meta::new(instrument_key, None))
            })
            .collect();

        Ok(Self { instrument_map })
    }
}

impl<InstrumentKey> Transformer for BybitOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone,
{
    type Error = DataError;
    type Input = BybitOrderBookMessage;
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

        // Initialise a sequencer when snapshot received from the exchange. We
        // return immediately because the snapshot message is always valid.
        if matches!(input.kind, BybitPayloadKind::Snapshot) {
            instrument.sequencer.replace(BybitOrderBookL2Sequencer {
                last_update_id: input.data.update_id,
            });

            return MarketIter::<InstrumentKey, OrderBookEvent>::from((
                BybitSpot::ID,
                instrument.key.clone(),
                input,
            ))
            .0;
        }

        // Could happen if we receive an update message before the snapshot
        let Some(sequencer) = &mut instrument.sequencer else {
            debug!("Update message received before initial Snapshot");
            return vec![];
        };

        // Drop any outdated updates & validate sequence for relevant updates
        let valid_update = match sequencer.validate_sequence(input) {
            Ok(Some(valid_update)) => valid_update,
            Ok(None) => return vec![],
            Err(error) => return vec![Err(error)],
        };

        MarketIter::<InstrumentKey, OrderBookEvent>::from((
            BybitSpot::ID,
            instrument.key.clone(),
            valid_update,
        ))
        .0
    }
}

#[derive(Debug)]
struct BybitOrderBookL2Sequencer {
    last_update_id: u64,
}

impl BybitOrderBookL2Sequencer {
    pub fn validate_sequence(
        &mut self,
        update: BybitOrderBookMessage,
    ) -> Result<Option<BybitOrderBookMessage>, DataError> {
        // Each new update_id should be `last_update_id + 1`
        if update.data.update_id != self.last_update_id + 1 {
            return Err(DataError::InvalidSequence {
                prev_last_update_id: self.last_update_id,
                first_update_id: update.data.update_id,
            });
        }

        // Update metadata
        self.last_update_id = update.data.update_id;

        Ok(Some(update))
    }
}
