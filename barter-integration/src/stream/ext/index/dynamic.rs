use crate::stream::ext::index::Indexer;
use derive_more::Constructor;
use futures::{Sink, Stream, ready};
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug)]
pub struct DynamicIndexer<I> {
    indexer: I,
}

pub trait IndexerExt<F>: Indexer {
    fn update(&mut self, update: F);
}

impl<I, F> IndexerExt<F> for DynamicIndexer<I>
where
    I: Indexer,
    F: FnMut(&mut Self),
{
    fn update(&mut self, mut update: F) {
        update(self)
    }
}

impl<I> Indexer for DynamicIndexer<I>
where
    I: Indexer,
{
    type Unindexed = I::Unindexed;
    type Indexed = I::Indexed;
    type Error = I::Error;

    fn index(&self, item: Self::Unindexed) -> Result<Self::Indexed, Self::Error> {
        self.indexer.index(item)
    }
}

pub trait Updateable {
    type Update;

    fn update(&mut self, update: Self::Update);
}

pub trait TryUpdateable {
    type Update;
    type Error;

    fn try_update(&mut self, update: Self::Update) -> Result<(), Self::Error>;
}

impl<I> DynamicIndexer<I> {}

#[derive(Debug, Constructor)]
#[pin_project]
pub struct DynamicIndexedStream<S, Indexer> {
    #[pin]
    socket: S,
    indexer: Indexer,
}

pub enum Event<Item, F> {
    InnerItem(Item),
    IndexUpdate(F),
}

impl<S, I> Stream for DynamicIndexedStream<S, I>
where
    S: Stream<Item = Event<I::Unindexed, I::Update>>,
    I: Indexer + Updateable,
{
    type Item = Result<I::Indexed, I::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            let next_ready = ready!(this.socket.as_mut().poll_next(cx));

            let Some(event) = next_ready else {
                return Poll::Ready(None);
            };

            match event {
                Event::InnerItem(item) => return Poll::Ready(Some(this.indexer.index(item))),
                Event::IndexUpdate(update) => {
                    this.indexer.update(update);
                }
            }
        }

        // match this.socket.poll_next(cx) {
        //     Poll::Ready(Some(item)) => Poll::Ready(Some(this.indexer.index(item))),
        //     Poll::Ready(None) => return Poll::Ready(None),
        //     Poll::Pending => return Poll::Pending,
        // }
    }
}

impl<S, Index, Item> Sink<Item> for DynamicIndexedStream<S, Index>
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
