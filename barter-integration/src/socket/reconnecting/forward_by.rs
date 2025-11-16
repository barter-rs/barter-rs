use futures::{Sink, Stream, ready};
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// Todo: Rust Docs
///  - FnPredicate determines which elements to forward and which to keep in Stream.
///  - If FnForward errors, Stream ends.
#[pin_project]
pub struct ForwardBy<S, FnPredicate, FnForward> {
    #[pin]
    socket: S,
    predicate: FnPredicate,
    forward: FnForward,
}

impl<S, FnPredicate, FnForward> ForwardBy<S, FnPredicate, FnForward> {
    pub fn new(socket: S, predicate: FnPredicate, forward: FnForward) -> Self {
        Self {
            socket,
            predicate,
            forward,
        }
    }
}

impl<S, A, B, FnPredicate, FnForward> Stream for ForwardBy<S, FnPredicate, FnForward>
where
    S: Stream,
    FnPredicate: Fn(S::Item) -> futures::future::Either<A, B>,
    FnForward: FnMut(A) -> Result<(), ()>,
{
    type Item = B;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            let next_ready = ready!(this.socket.as_mut().poll_next(cx));

            let Some(item) = next_ready else {
                return Poll::Ready(None);
            };

            match (this.predicate)(item) {
                futures::future::Either::Left(left) => {
                    if (this.forward)(left).is_err() {
                        return Poll::Ready(None);
                    } else {
                        // Initiate next poll_next immediately
                    }
                }
                futures::future::Either::Right(right) => return Poll::Ready(Some(right)),
            }
        }
    }
}

impl<S, FnPredicate, FnForward, Item> Sink<Item> for ForwardBy<S, FnPredicate, FnForward>
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
