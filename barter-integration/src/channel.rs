use crate::Unrecoverable;
use derive_more::{Constructor, Display};
use futures::{Sink, Stream};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    pin::Pin,
    task::{Context, Poll},
};
use tracing::warn;

pub trait Tx
where
    Self: Debug + Clone + Send,
{
    type Item;
    type Error: Unrecoverable + Debug;
    fn send<Item: Into<Self::Item>>(&self, item: Item) -> Result<(), Self::Error>;
}

/// Convenience type that holds the [`UnboundedTx`] and [`UnboundedRx`].
#[derive(Debug)]
pub struct Channel<T> {
    pub tx: UnboundedTx<T>,
    pub rx: UnboundedRx<T>,
}

impl<T> Channel<T> {
    /// Construct a new unbounded [`Channel`].
    pub fn new() -> Self {
        let (tx, rx) = mpsc_unbounded();
        Self { tx, rx }
    }
}

impl<T> Default for Channel<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct UnboundedTx<T> {
    pub tx: tokio::sync::mpsc::UnboundedSender<T>,
}

impl<T> UnboundedTx<T> {
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<T>) -> Self {
        Self { tx }
    }
}

impl<T> Tx for UnboundedTx<T>
where
    T: Debug + Clone + Send,
{
    type Item = T;
    type Error = tokio::sync::mpsc::error::SendError<T>;

    fn send<Item: Into<Self::Item>>(&self, item: Item) -> Result<(), Self::Error> {
        self.tx.send(item.into())
    }
}

impl<T> Unrecoverable for tokio::sync::mpsc::error::SendError<T> {
    fn is_unrecoverable(&self) -> bool {
        true
    }
}

impl<T> Sink<T> for UnboundedTx<T> {
    type Error = tokio::sync::mpsc::error::SendError<T>;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // UnboundedTx is always ready
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        self.tx.send(item)
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // UnboundedTx does not buffer, so no flushing is required
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // UnboundedTx requires no closing logic
        Poll::Ready(Ok(()))
    }
}

#[derive(Debug, Constructor)]
pub struct UnboundedRx<T> {
    pub rx: tokio::sync::mpsc::UnboundedReceiver<T>,
}

impl<T> Iterator for UnboundedRx<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.rx.try_recv() {
                Ok(event) => break Some(event),
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => continue,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break None,
            }
        }
    }
}

impl<T> UnboundedRx<T> {
    pub fn into_stream(self) -> tokio_stream::wrappers::UnboundedReceiverStream<T> {
        tokio_stream::wrappers::UnboundedReceiverStream::new(self.rx)
    }
}

impl<T> Stream for UnboundedRx<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
pub struct ChannelTxDroppable<ChannelTx> {
    pub state: ChannelState<ChannelTx>,
}

impl<ChannelTx> ChannelTxDroppable<ChannelTx> {
    pub fn new(tx: ChannelTx) -> Self {
        Self {
            state: ChannelState::Active(tx),
        }
    }

    pub fn new_disabled() -> Self {
        Self {
            state: ChannelState::Disabled,
        }
    }

    pub fn disable(&mut self) {
        self.state = ChannelState::Disabled
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize, Display)]
pub enum ChannelState<Tx> {
    Active(Tx),
    Disabled,
}

impl<ChannelTx> ChannelTxDroppable<ChannelTx>
where
    ChannelTx: Tx,
{
    pub fn send(&mut self, item: ChannelTx::Item) {
        let ChannelState::Active(tx) = &self.state else {
            return;
        };

        if tx.send(item).is_err() {
            let name = std::any::type_name::<ChannelTx::Item>();
            warn!(
                name,
                "ChannelTxDroppable receiver dropped - items will no longer be sent"
            );
            self.state = ChannelState::Disabled
        }
    }
}
pub fn mpsc_unbounded<T>() -> (UnboundedTx<T>, UnboundedRx<T>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    (UnboundedTx::new(tx), UnboundedRx::new(rx))
}
