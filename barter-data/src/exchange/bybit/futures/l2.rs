use super::super::book::{l2::BybitBookUpdater, l2::BybitOrderBookL2};
use crate::{
    error::DataError,
    subscription::book::OrderBook,
    transformer::book::{InstrumentOrderBook, OrderBookUpdater},
};
use async_trait::async_trait;
use barter_integration::{model::instrument::Instrument, protocol::websocket::WsMessage};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BybitPerpetualsBookUpdater(BybitBookUpdater);

impl BybitPerpetualsBookUpdater {
    pub fn new(last_update_id: u64, last_sequence: u64) -> Self {
        Self(BybitBookUpdater::new(last_update_id, last_sequence))
    }
}

#[async_trait]
impl OrderBookUpdater for BybitPerpetualsBookUpdater {
    type OrderBook = OrderBook;
    type Update = BybitOrderBookL2;

    async fn init<Exchange, Kind>(
        sender: mpsc::UnboundedSender<WsMessage>,
        instrument: Instrument,
    ) -> Result<InstrumentOrderBook<Instrument, Self>, DataError>
    where
        Exchange: Send,
        Kind: Send,
    {
        let inner_result = BybitBookUpdater::init::<Exchange, Kind>(sender, instrument).await?;
        Ok(InstrumentOrderBook {
            instrument: inner_result.instrument,
            updater: Self(inner_result.updater),
            book: inner_result.book,
        })
    }

    fn update(
        &mut self,
        book: &mut Self::OrderBook,
        update: Self::Update,
    ) -> Result<Option<Self::OrderBook>, DataError> {
        self.0.update(book, update)
    }
}
