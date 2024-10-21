use crate::{
    error::DataError,
    streams::consumer::MarketStreamResult,
    subscription::{
        book::{OrderBookEvent, OrderBookL1},
        candle::Candle,
        liquidation::Liquidation,
        trade::PublicTrade,
    },
};
use barter_instrument::{exchange::ExchangeId, instrument::Instrument};
use chrono::{DateTime, Utc};
use derive_more::From;
use serde::{Deserialize, Serialize};

/// Convenient new type containing a collection of [`MarketEvent<T>`](MarketEvent)s.
#[derive(Debug)]
pub struct MarketIter<InstrumentKey, T>(pub Vec<Result<MarketEvent<InstrumentKey, T>, DataError>>);

impl<InstrumentKey, T> FromIterator<Result<MarketEvent<InstrumentKey, T>, DataError>>
    for MarketIter<InstrumentKey, T>
{
    fn from_iter<Iter>(iter: Iter) -> Self
    where
        Iter: IntoIterator<Item = Result<MarketEvent<InstrumentKey, T>, DataError>>,
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
pub struct MarketEvent<InstrumentKey = Instrument, T = DataKind> {
    pub time_exchange: DateTime<Utc>,
    pub time_received: DateTime<Utc>,
    pub exchange: ExchangeId,
    pub instrument: InstrumentKey,
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

impl<InstrumentKey> From<MarketStreamResult<InstrumentKey, PublicTrade>>
    for MarketStreamResult<InstrumentKey, DataKind>
{
    fn from(value: MarketStreamResult<InstrumentKey, PublicTrade>) -> Self {
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

impl<InstrumentKey> From<MarketStreamResult<InstrumentKey, OrderBookL1>>
    for MarketStreamResult<InstrumentKey, DataKind>
{
    fn from(value: MarketStreamResult<InstrumentKey, OrderBookL1>) -> Self {
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

impl<InstrumentKey> From<MarketStreamResult<InstrumentKey, OrderBookEvent>>
    for MarketStreamResult<InstrumentKey, DataKind>
{
    fn from(value: MarketStreamResult<InstrumentKey, OrderBookEvent>) -> Self {
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

impl<InstrumentKey> From<MarketStreamResult<InstrumentKey, Candle>>
    for MarketStreamResult<InstrumentKey, DataKind>
{
    fn from(value: MarketStreamResult<InstrumentKey, Candle>) -> Self {
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

impl<InstrumentKey> From<MarketStreamResult<InstrumentKey, Liquidation>>
    for MarketStreamResult<InstrumentKey, DataKind>
{
    fn from(value: MarketStreamResult<InstrumentKey, Liquidation>) -> Self {
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
