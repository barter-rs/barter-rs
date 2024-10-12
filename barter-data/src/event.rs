use crate::streams::consumer::MarketStreamEvent;
use crate::subscription::book::OrderBookEvent;
use crate::{
    error::DataError,
    subscription::{
        book::OrderBookL1, candle::Candle, liquidation::Liquidation, trade::PublicTrade,
    },
};
use barter_integration::model::{instrument::Instrument, Exchange};
use chrono::{DateTime, Utc};
use derive_more::From;
use serde::{Deserialize, Serialize};

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

impl<InstrumentKey, T> MarketEvent<InstrumentKey, T> {
    pub fn map_kind<F, O>(self, op: F) -> MarketEvent<InstrumentKey, O>
    where
        F: FnOnce(T) -> O,
    {
        MarketEvent {
            time_exchange: self.time_exchange,
            time_received: self.time_received,
            exchange: self.exchange,
            instrument: self.instrument,
            kind: op(self.kind),
        }
    }
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
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize, From)]
pub enum DataKind {
    Trade(PublicTrade),
    OrderBookL1(OrderBookL1),
    OrderBook(OrderBookEvent),
    Candle(Candle),
    Liquidation(Liquidation),
}

impl<InstrumentKey> From<MarketStreamEvent<InstrumentKey, PublicTrade>>
    for MarketStreamEvent<InstrumentKey, DataKind>
{
    fn from(value: MarketStreamEvent<InstrumentKey, PublicTrade>) -> Self {
        value.map_ok(MarketEvent::from)
    }
}

impl<InstrumentKey> From<MarketEvent<InstrumentKey, PublicTrade>>
    for MarketEvent<InstrumentKey, DataKind>
{
    fn from(value: MarketEvent<InstrumentKey, PublicTrade>) -> Self {
        value.map_kind(PublicTrade::into)
    }
}

impl<InstrumentKey> From<MarketStreamEvent<InstrumentKey, OrderBookL1>>
    for MarketStreamEvent<InstrumentKey, DataKind>
{
    fn from(value: MarketStreamEvent<InstrumentKey, OrderBookL1>) -> Self {
        value.map_ok(MarketEvent::from)
    }
}

impl<InstrumentKey> From<MarketEvent<InstrumentKey, OrderBookL1>>
    for MarketEvent<InstrumentKey, DataKind>
{
    fn from(value: MarketEvent<InstrumentKey, OrderBookL1>) -> Self {
        value.map_kind(OrderBookL1::into)
    }
}

impl<InstrumentKey> From<MarketStreamEvent<InstrumentKey, OrderBookEvent>>
    for MarketStreamEvent<InstrumentKey, DataKind>
{
    fn from(value: MarketStreamEvent<InstrumentKey, OrderBookEvent>) -> Self {
        value.map_ok(MarketEvent::from)
    }
}

impl<InstrumentKey> From<MarketEvent<InstrumentKey, OrderBookEvent>>
    for MarketEvent<InstrumentKey, DataKind>
{
    fn from(value: MarketEvent<InstrumentKey, OrderBookEvent>) -> Self {
        value.map_kind(OrderBookEvent::into)
    }
}

impl<InstrumentKey> From<MarketStreamEvent<InstrumentKey, Candle>>
    for MarketStreamEvent<InstrumentKey, DataKind>
{
    fn from(value: MarketStreamEvent<InstrumentKey, Candle>) -> Self {
        value.map_ok(MarketEvent::from)
    }
}

impl<InstrumentKey> From<MarketEvent<InstrumentKey, Candle>>
    for MarketEvent<InstrumentKey, DataKind>
{
    fn from(value: MarketEvent<InstrumentKey, Candle>) -> Self {
        value.map_kind(Candle::into)
    }
}

impl<InstrumentKey> From<MarketStreamEvent<InstrumentKey, Liquidation>>
    for MarketStreamEvent<InstrumentKey, DataKind>
{
    fn from(value: MarketStreamEvent<InstrumentKey, Liquidation>) -> Self {
        value.map_ok(MarketEvent::from)
    }
}

impl<InstrumentKey> From<MarketEvent<InstrumentKey, Liquidation>>
    for MarketEvent<InstrumentKey, DataKind>
{
    fn from(value: MarketEvent<InstrumentKey, Liquidation>) -> Self {
        value.map_kind(Liquidation::into)
    }
}
