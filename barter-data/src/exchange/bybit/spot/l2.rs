use super::super::book::{l2::BybitOrderBookL2, l2::BybitOrderBookL2Type};
use crate::{
    error::DataError,
    subscription::book::{Level, OrderBook, OrderBookSide},
    transformer::book::{InstrumentOrderBook, OrderBookUpdater},
};
use async_trait::async_trait;
use barter_integration::{
    model::{instrument::Instrument, Side},
    protocol::websocket::WsMessage,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Docs <https://bybit-exchange.github.io/docs/v5/websocket/public/orderbook>
///
/// Process snapshot/delta. Excerpt from the docs:
///
/// To process snapshot and delta messages, please follow these rules:
///
/// Once you have subscribed successfully, you will receive a snapshot.
/// The WebSocket will keep pushing delta messages every time the orderbook changes.
/// If you receive a new snapshot message, you will have to reset your local orderbook.
/// If there is a problem on Bybit's end, a snapshot will be re-sent, which is guaranteed to contain the latest data.
///
/// To apply delta updates:
///
/// If you receive an amount that is 0, delete the entry.
/// If you receive an amount that does not exist, insert it.
/// If the entry exists, you simply update the value.
///
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BybitSpotBookUpdater {
    pub last_update_id: u64,
    pub last_sequence: u64,
}

impl BybitSpotBookUpdater {
    pub fn new(last_update_id: u64, last_sequence: u64) -> Self {
        Self {
            last_update_id,
            last_sequence,
        }
    }

    pub fn validate_next_update(&self, update: &BybitOrderBookL2) -> Result<(), DataError> {
        if update.update_type == BybitOrderBookL2Type::Snapshot {
            return Ok(());
        }
        if update.data.u == self.last_update_id + 1 {
            Ok(())
        } else {
            Err(DataError::InvalidSequence {
                prev_last_update_id: self.last_update_id,
                first_update_id: update.data.u,
            })
        }
    }
}

#[async_trait]
impl OrderBookUpdater for BybitSpotBookUpdater {
    type OrderBook = OrderBook;
    type Update = BybitOrderBookL2;

    async fn init<Exchange, Kind>(
        _: mpsc::UnboundedSender<WsMessage>,
        instrument: Instrument,
    ) -> Result<InstrumentOrderBook<Instrument, Self>, DataError>
    where
        Exchange: Send,
        Kind: Send,
    {
        // Just a duumy orderbook because we expect to get the snapshot thru websocket
        Ok(InstrumentOrderBook {
            instrument,
            updater: Self::new(0, 0),
            book: OrderBook {
                last_update_time: DateTime::<Utc>::MIN_UTC,
                bids: OrderBookSide::new(Side::Buy, Vec::<Level>::new()),
                asks: OrderBookSide::new(Side::Sell, Vec::<Level>::new()),
            },
        })
    }

    fn update(
        &mut self,
        book: &mut Self::OrderBook,
        update: Self::Update,
    ) -> Result<Option<Self::OrderBook>, DataError> {
        if update.data.u <= self.last_update_id {
            return Ok(None);
        }

        self.validate_next_update(&update)?;

        match update.update_type {
            BybitOrderBookL2Type::Snapshot => {
                book.last_update_time = update.ts;
                book.bids = OrderBookSide::new(Side::Buy, update.data.b);
                book.asks = OrderBookSide::new(Side::Sell, update.data.a);
            }
            BybitOrderBookL2Type::Delta => {
                book.last_update_time = update.ts;
                book.bids.upsert(update.data.b);
                book.asks.upsert(update.data.a);
            }
        }

        // Update OrderBookUpdater metadata
        self.last_update_id = update.data.u;
        self.last_sequence = update.data.seq;

        Ok(Some(book.snapshot()))
    }
}
