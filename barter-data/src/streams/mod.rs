use self::builder::{multi::MultiStreamBuilder, StreamBuilder};
use crate::{exchange::ExchangeId, subscription::SubscriptionKind};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamMap};

/// Defines the [`StreamBuilder`](builder::StreamBuilder) and
/// [`MultiStreamBuilder`](builder::multi::MultiStreamBuilder) APIs for ergonomically initialising
/// [`MarketStream`](super::MarketStream) [`Streams`].
pub mod builder;

/// Central consumer loop functionality used by the [`StreamBuilder`](builder::StreamBuilder) to
/// to drive a re-connecting [`MarketStream`](super::MarketStream).
pub mod consumer;

/// Ergonomic collection of exchange [`MarketEvent<T>`](crate::event::MarketEvent) receivers.
#[derive(Debug)]
pub struct Streams<T> {
    pub streams: HashMap<ExchangeId, mpsc::UnboundedReceiver<T>>,
}

impl<T> Streams<T> {
    /// Construct a [`StreamBuilder`] for configuring new
    /// [`MarketEvent<SubscriptionKind::Event>`](crate::event::MarketEvent) [`Streams`].
    pub fn builder<Kind>() -> StreamBuilder<Kind>
    where
        Kind: SubscriptionKind,
    {
        StreamBuilder::<Kind>::new()
    }

    /// Construct a [`MultiStreamBuilder`] for configuring new
    /// [`MarketEvent<T>`](crate::event::MarketEvent) [`Streams`].
    pub fn builder_multi() -> MultiStreamBuilder<T> {
        MultiStreamBuilder::<T>::new()
    }

    /// Remove an exchange [`mpsc::UnboundedReceiver`] from the [`Streams`] `HashMap`.
    pub fn select(&mut self, exchange: ExchangeId) -> Option<mpsc::UnboundedReceiver<T>> {
        self.streams.remove(&exchange)
    }

    /// Join all exchange [`mpsc::UnboundedReceiver`] streams into a unified
    /// [`mpsc::UnboundedReceiver`].
    pub async fn join(self) -> mpsc::UnboundedReceiver<T>
    where
        T: Send + 'static,
    {
        let (joined_tx, joined_rx) = mpsc::unbounded_channel();

        for mut exchange_rx in self.streams.into_values() {
            let joined_tx = joined_tx.clone();
            tokio::spawn(async move {
                while let Some(event) = exchange_rx.recv().await {
                    let _ = joined_tx.send(event);
                }
            });
        }

        joined_rx
    }

    /// Join all exchange [`mpsc::UnboundedReceiver`] streams into a unified [`StreamMap`].
    pub async fn join_map(self) -> StreamMap<ExchangeId, UnboundedReceiverStream<T>> {
        self.streams
            .into_iter()
            .fold(StreamMap::new(), |mut map, (exchange, rx)| {
                map.insert(exchange, UnboundedReceiverStream::new(rx));
                map
            })
    }
}
