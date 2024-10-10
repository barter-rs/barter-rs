use crate::{
    error::DataError,
    subscription::{
        book::{OrderBookL1},
        candle::Candle,
        liquidation::Liquidation,
        trade::PublicTrade,
    },
};
use barter_integration::model::{instrument::Instrument, Exchange};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::subscription::book::OrderBookEvent;

/// Convenient new type containing a collection of [`MarketEvent<T>`](MarketEvent)s.
#[derive(Debug)]
pub struct MarketIter<InstrumentId, T>(pub Vec<Result<MarketEvent<InstrumentId, T>, DataError>>);

impl<InstrumentId, T> FromIterator<Result<MarketEvent<InstrumentId, T>, DataError>>
    for MarketIter<InstrumentId, T>
{
    fn from_iter<Iter>(iter: Iter) -> Self
    where
        Iter: IntoIterator<Item = Result<MarketEvent<InstrumentId, T>, DataError>>,
    {
        Self(iter.into_iter().collect())
    }
}

/// Normalised Barter [`MarketEvent<T>`](Self) wrapping the `T` data variant in metadata.
///
/// Note: `T` can be an enum such as the [`DataKind`] if required.
///
/// See [`crate::subscription`] for all existing Barter Market event variants.
///
/// ### Examples
/// - [`MarketEvent<PublicTrade>`](PublicTrade)
/// - [`MarketEvent<OrderBookL1>`](OrderBookL1)
/// - [`MarketEvent<DataKind>`](DataKind)
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct MarketEvent<InstrumentId = Instrument, T = DataKind> {
    pub time_exchange: DateTime<Utc>,
    pub time_received: DateTime<Utc>,
    pub exchange: Exchange,
    pub instrument: InstrumentId,
    pub kind: T,
}

/// Available kinds of normalised Barter [`MarketEvent<T>`](MarketEvent).
///
/// ### Notes
/// - [`Self`] is only used as the [`MarketEvent<DataKind>`](MarketEvent) `Output` when combining
///   several [`Streams<SubscriptionKind::Event>`](crate::streams::Streams) using the
///   [`MultiStreamBuilder<Output>`](crate::streams::builder::multi::MultiStreamBuilder), or via
///   the [`DynamicStreams::select_all`](crate::streams::builder::dynamic::DynamicStreams) method.
/// - [`Self`] is purposefully not supported in any
///   [`Subscription`](crate::subscription::Subscription)s directly, it is only used to
///   make ergonomic [`Streams`](crate::streams::Streams) containing many
///   [`MarketEvent<T>`](MarketEvent) kinds.
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub enum DataKind {
    Trade(PublicTrade),
    OrderBookL1(OrderBookL1),
    OrderBook(OrderBookEvent),
    Candle(Candle),
    Liquidation(Liquidation),
}

impl<InstrumentId> From<MarketEvent<InstrumentId, PublicTrade>>
    for MarketEvent<InstrumentId, DataKind>
{
    fn from(event: MarketEvent<InstrumentId, PublicTrade>) -> Self {
        Self {
            time_exchange: event.time_exchange,
            time_received: event.time_received,
            exchange: event.exchange,
            instrument: event.instrument,
            kind: DataKind::Trade(event.kind),
        }
    }
}

impl<InstrumentId> From<MarketEvent<InstrumentId, OrderBookL1>>
    for MarketEvent<InstrumentId, DataKind>
{
    fn from(event: MarketEvent<InstrumentId, OrderBookL1>) -> Self {
        Self {
            time_exchange: event.time_exchange,
            time_received: event.time_received,
            exchange: event.exchange,
            instrument: event.instrument,
            kind: DataKind::OrderBookL1(event.kind),
        }
    }
}

impl<InstrumentId> From<MarketEvent<InstrumentId, OrderBookEvent>>
    for MarketEvent<InstrumentId, DataKind>
{
    fn from(event: MarketEvent<InstrumentId, OrderBookEvent>) -> Self {
        Self {
            time_exchange: event.time_exchange,
            time_received: event.time_received,
            exchange: event.exchange,
            instrument: event.instrument,
            kind: DataKind::OrderBook(event.kind),
        }
    }
}

impl<InstrumentId> From<MarketEvent<InstrumentId, Candle>> for MarketEvent<InstrumentId, DataKind> {
    fn from(event: MarketEvent<InstrumentId, Candle>) -> Self {
        Self {
            time_exchange: event.time_exchange,
            time_received: event.time_received,
            exchange: event.exchange,
            instrument: event.instrument,
            kind: DataKind::Candle(event.kind),
        }
    }
}

impl<InstrumentId> From<MarketEvent<InstrumentId, Liquidation>>
    for MarketEvent<InstrumentId, DataKind>
{
    fn from(event: MarketEvent<InstrumentId, Liquidation>) -> Self {
        Self {
            time_exchange: event.time_exchange,
            time_received: event.time_received,
            exchange: event.exchange,
            instrument: event.instrument,
            kind: DataKind::Liquidation(event.kind),
        }
    }
}
