use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

pub trait Tx
where
    Self: Clone + Send,
{
    type Item;
    type Error;
    fn send(&self, item: Self::Item) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize, Display)]
pub enum ChannelState<Tx> {
    Active(Tx),
    Disabled,
}

#[derive(Debug, Clone)]
pub struct UnboundedTx<T, Error> {
    pub tx: tokio::sync::mpsc::UnboundedSender<T>,
    phantom: PhantomData<Error>,
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

#[derive(Debug)]
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

pub fn mpsc_unbounded<T, Error>() -> (UnboundedTx<T, Error>, UnboundedRx<T>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    (
        UnboundedTx {
            tx,
            phantom: PhantomData,
        },
        UnboundedRx { rx },
    )
}
