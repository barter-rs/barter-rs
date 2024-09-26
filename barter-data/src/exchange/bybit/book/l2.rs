use crate::{
    error::DataError,
    exchange::{bybit::channel::BybitChannel, subscription::ExchangeSub},
    subscription::book::{Level, OrderBook, OrderBookSide},
    transformer::book::{InstrumentOrderBook, OrderBookUpdater},
    Identifier,
};
use async_trait::async_trait;
use barter_integration::{
    model::{instrument::Instrument, Side, SubscriptionId},
    protocol::websocket::WsMessage,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitOrderBookL2 {
    pub topic: String,
    #[serde(rename = "type")]
    pub update_type: BybitOrderBookL2Type,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub ts: DateTime<Utc>,
    pub data: BybitOrderBookL2Data,
    pub cts: u64,
}

impl Identifier<Option<SubscriptionId>> for BybitOrderBookL2 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.data.subscription_id.clone())
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum BybitOrderBookL2Type {
    #[serde(alias = "snapshot")]
    Snapshot,
    #[serde(alias = "delta")]
    Delta,
}

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitOrderBookL2Data {
    #[serde(alias = "s", deserialize_with = "de_ob_l2_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(deserialize_with = "de_ob_l2_levels")]
    pub b: Vec<BybitLevel>,
    #[serde(deserialize_with = "de_ob_l2_levels")]
    pub a: Vec<BybitLevel>,
    pub u: u64,
    pub seq: u64,
}

/// Deserialize a [`BybitOrderBookL2`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`].
///
/// eg/ "orderbook.50.BTCUSDT"
pub fn de_ob_l2_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((BybitChannel::ORDER_BOOK_L2, market)).id())
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitLevel(pub f64, pub f64);

impl From<BybitLevel> for Level {
    fn from(bybit_level: BybitLevel) -> Self {
        Level {
            price: bybit_level.0,
            amount: bybit_level.1,
        }
    }
}

pub fn de_ob_l2_levels<'de, D>(deserializer: D) -> Result<Vec<BybitLevel>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let levels = Vec::<[&str; 2]>::deserialize(deserializer)?;

    levels
        .into_iter()
        .map(|[price, amount]| {
            Ok(BybitLevel(
                price.parse().map_err(serde::de::Error::custom)?,
                amount.parse().map_err(serde::de::Error::custom)?,
            ))
        })
        .collect()
}

impl From<BybitOrderBookL2> for OrderBook {
    fn from(snapshot: BybitOrderBookL2) -> Self {
        Self {
            last_update_time: snapshot.ts,
            bids: OrderBookSide::new(Side::Buy, snapshot.data.b),
            asks: OrderBookSide::new(Side::Sell, snapshot.data.a),
        }
    }
}

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
pub struct BybitBookUpdater {
    pub last_update_id: u64,
    pub last_sequence: u64,
}

impl BybitBookUpdater {
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
impl OrderBookUpdater for BybitBookUpdater {
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
        // Just a dummy orderbook because we expect to get the snapshot thru websocket
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
