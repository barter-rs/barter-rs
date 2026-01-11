use crate::{
    NoInitialSnapshots,
    exchange::{ExchangeServer, StreamSelector},
    instrument::InstrumentData,
    subscription::{book::OrderBooksL1, trade::PublicTrades},
    transformer::stateless::StatelessTransformer,
};
use barter_instrument::exchange::ExchangeId;
use super::{KrakenExchange, KrakenWsStream};
use self::{
    book::l1::KrakenOrderBookL1, trade::KrakenTrades,
};

pub mod book;
pub mod trade;

/// [`KrakenSpot`] execution server.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct KrakenServerSpot;

impl ExchangeServer for KrakenServerSpot {
    const ID: ExchangeId = ExchangeId::Kraken;

    fn websocket_url() -> &'static str {
        "wss://ws.kraken.com/"
    }
}

/// Type alias for [`Kraken`](super::Kraken) Spot exchange configuration.
pub type KrakenSpot = KrakenExchange<KrakenServerSpot>;

impl<Instrument> StreamSelector<Instrument, PublicTrades> for KrakenSpot
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream =
        KrakenWsStream<StatelessTransformer<Self, Instrument::Key, PublicTrades, KrakenTrades>>;
}

impl<Instrument> StreamSelector<Instrument, OrderBooksL1> for KrakenSpot
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = KrakenWsStream<
        StatelessTransformer<Self, Instrument::Key, OrderBooksL1, KrakenOrderBookL1>,
    >;
}
