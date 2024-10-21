use self::builder::{multi::MultiStreamBuilder, StreamBuilder};
use crate::subscription::SubscriptionKind;
use barter_instrument::exchange::ExchangeId;
use fnv::FnvHashMap;
use futures::Stream;
use futures_util::stream::select_all;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

/// Defines the [`StreamBuilder`] and [`MultiStreamBuilder`] APIs for ergonomically initialising
/// [`MarketStream`](super::MarketStream) [`Streams`].
pub mod builder;

/// Central consumer loop functionality used by the [`StreamBuilder`] to
/// drive a re-connecting [`MarketStream`](super::MarketStream).
pub mod consumer;

/// Defines a [`ReconnectingStream`] and associated logic for generating an auto reconnecting
/// `Stream`.
pub mod reconnect;

/// Ergonomic collection of exchange [`MarketEvent<T>`](crate::event::MarketEvent) receivers.
#[derive(Debug)]
pub struct Streams<T> {
    pub streams: FnvHashMap<ExchangeId, mpsc::UnboundedReceiver<T>>,
}

impl<T> Streams<T> {
    /// Construct a [`StreamBuilder`] for configuring new
    /// [`MarketEvent<SubscriptionKind::Event>`](crate::event::MarketEvent) [`Streams`].
    pub fn builder<InstrumentKey, Kind>() -> StreamBuilder<InstrumentKey, Kind>
    where
        Kind: SubscriptionKind,
    {
        StreamBuilder::<InstrumentKey, Kind>::new()
    }

    /// Construct a [`MultiStreamBuilder`] for configuring new
    /// [`MarketEvent<T>`](crate::event::MarketEvent) [`Streams`].
    pub fn builder_multi() -> MultiStreamBuilder<T> {
        MultiStreamBuilder::<T>::new()
    }

    /// Remove an exchange [`mpsc::UnboundedReceiver`] from the [`Streams`] `HashMap`.
    pub fn select(&mut self, exchange: ExchangeId) -> Option<impl Stream<Item = T> + '_> {
        self.streams
            .remove(&exchange)
            .map(UnboundedReceiverStream::new)
    }

    /// Select and merge every exchange `Stream` using [`select_all`].
    pub fn select_all(self) -> impl Stream<Item = T> {
        let all = self.streams.into_values().map(UnboundedReceiverStream::new);

        select_all(all)
    }
}
