use futures::{Sink, Stream, ready};
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug)]
#[pin_project]
pub struct ForwardCloneBy<S, FnPredicate, FnForward> {
    #[pin]
    socket: S,
    predicate: FnPredicate,
    forward: FnForward,
}

impl<S, FnPredicate, FnForward> ForwardCloneBy<S, FnPredicate, FnForward> {
    pub fn new(socket: S, predicate: FnPredicate, forward: FnForward) -> Self {
        Self {
            socket,
            predicate,
            forward,
        }
    }
}

impl<S, FnPredicate, FnForward> Stream for ForwardCloneBy<S, FnPredicate, FnForward>
where
    S: Stream,
    S::Item: Clone,
    FnPredicate: FnMut(&S::Item) -> bool,
    FnForward: FnMut(S::Item) -> Result<(), ()>,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        let next_ready = ready!(this.socket.as_mut().poll_next(cx));

        let Some(item) = next_ready else {
            return Poll::Ready(None);
        };

        if (this.predicate)(&item) && (this.forward)(item.clone()).is_err() {
            return Poll::Ready(None);
        }

        Poll::Ready(Some(item))
    }
}

impl<S, FnPredicate, FnForward, Item> Sink<Item> for ForwardCloneBy<S, FnPredicate, FnForward>
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
