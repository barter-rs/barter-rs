use derive_more::Constructor;
use serde::{Deserialize, Serialize};

use crate::{
    NoInitialSnapshots,
    exchange::{
        StreamSelector,
        bitstamp::{
            BitstampSpot, BitstampWsStream,
            book::l2::{BitstampOrderBooksL2SnapshotFetcher, BitstampOrderBooksL2Transformer},
            message::BitstampPayload,
        },
    },
    instrument::InstrumentData,
    subscription::book::OrderBooksL2,
};

mod l2;
mod message;

impl<Instrument> StreamSelector<Instrument, OrderBooksL2> for BitstampSpot
where
    Instrument: InstrumentData,
{
    type SnapFetcher = BitstampOrderBooksL2SnapshotFetcher;
    type Stream = BitstampWsStream<BitstampOrderBooksL2Transformer<Instrument::Key>>;
}

#[derive(Debug, Constructor)]
pub struct BitstampOrderBookL2Meta<InstrumentKey, Sequencer> {
    pub key: InstrumentKey,
    pub sequencer: Sequencer,
}
