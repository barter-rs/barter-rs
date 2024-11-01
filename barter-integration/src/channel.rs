use derive_more::{Constructor, Display};
use futures::Sink;
use serde::{Deserialize, Serialize};
use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use tracing::warn;

pub trait Tx
where
    Self: Clone + Send,
{
    type Item;
    type Error;
    fn send(&self, item: Self::Item) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone)]
pub struct UnboundedTx<T, Error> {
    pub tx: tokio::sync::mpsc::UnboundedSender<T>,
    phantom: PhantomData<Error>,
}

impl<T, Error> UnboundedTx<T, Error> {
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<T>) -> Self {
        Self {
            tx,
            phantom: PhantomData,
        }
    }
}

impl<T, Error> Tx for UnboundedTx<T, Error>
where
    T: Clone + Send,
    Error: From<tokio::sync::mpsc::error::SendError<T>> + Clone + Send,
{
    type Item = T;
    type Error = Error;

    fn send(&self, item: Self::Item) -> Result<(), Self::Error> {
        self.tx.send(item).map_err(Error::from)
    }
}

impl<T, Error> Sink<T> for UnboundedTx<T, Error>
where
    Error: From<tokio::sync::mpsc::error::SendError<T>>,
{
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // UnboundedTx is always ready
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        self.tx.send(item).map_err(Error::from)
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
pub fn mpsc_unbounded<T, Error>() -> (UnboundedTx<T, Error>, UnboundedRx<T>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    (UnboundedTx::new(tx), UnboundedRx::new(rx))
}
