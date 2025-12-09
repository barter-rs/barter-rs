use barter_instrument::index::error::IndexError;
use derive_more::Constructor;
use futures::{Sink, Stream};
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub trait Indexer {
    type Unindexed;
    type Indexed;
    type Error;
    fn index(&self, item: Self::Unindexed) -> Result<Self::Indexed, Self::Error>;
}

#[derive(Debug, Constructor)]
#[pin_project]
pub struct IndexedStream<S, Indexer> {
    #[pin]
    socket: S,
    indexer: Indexer,
}

impl<S, Index> Stream for IndexedStream<S, Index>
where
    S: Stream,
    Index: Indexer<Unindexed = S::Item>,
{
    type Item = Result<Index::Indexed, Index::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.socket.poll_next(cx) {
            Poll::Ready(Some(item)) => Poll::Ready(Some(this.indexer.index(item))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<S, Index, Item> Sink<Item> for IndexedStream<S, Index>
where
    S: Sink<Item>,
{
    type Error = S::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().socket.poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        self.project().socket.start_send(item)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().socket.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().socket.poll_close(cx)
    }
}
