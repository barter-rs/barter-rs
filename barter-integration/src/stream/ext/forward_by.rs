use futures::{Sink, Stream, ready};
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// Stream adapter that forwards matching items and yields non-matching items.
///
/// Uses predicate to split items: Left items are forwarded, Right items are yielded.
#[derive(Debug)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::ext::BarterStreamExt;
    use futures::{StreamExt, future::Either};
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use tokio_test::{assert_pending, assert_ready_eq};

    #[tokio::test]
    async fn test_forward_by() {
        let waker = futures::task::noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        let (tx, rx) = mpsc::unbounded_channel::<i32>();
        let rx = UnboundedReceiverStream::new(rx);

        let (forward_tx, mut forward_rx) = mpsc::unbounded_channel::<i32>();

        let mut stream = rx.forward_by(
            |item| {
                if item % 2 == 0 {
                    Either::Left(item)
                } else {
                    Either::Right(item)
                }
            },
            move |item| forward_tx.send(item).map_err(|_| ()),
        );

        assert_pending!(stream.poll_next_unpin(&mut cx));

        tx.send(1).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(1));
        assert!(forward_rx.try_recv().is_err());

        tx.send(2).unwrap();
        assert_pending!(stream.poll_next_unpin(&mut cx));
        assert_eq!(forward_rx.try_recv().unwrap(), 2);

        tx.send(3).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(3));

        tx.send(4).unwrap();
        tx.send(5).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(5));
        assert_eq!(forward_rx.try_recv().unwrap(), 4);

        drop(tx);
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), None);
    }

    #[tokio::test]
    async fn test_forward_by_terminates_on_forward_error() {
        let waker = futures::task::noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        let (tx, rx) = mpsc::unbounded_channel::<i32>();
        let rx = UnboundedReceiverStream::new(rx);

        let mut stream = rx.forward_by(
            |item| {
                if item % 2 == 0 {
                    Either::Left(item)
                } else {
                    Either::Right(item)
                }
            },
            |item| if item == 4 { Err(()) } else { Ok(()) },
        );

        tx.send(1).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(1));

        tx.send(2).unwrap();
        assert_pending!(stream.poll_next_unpin(&mut cx));

        tx.send(4).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), None);
    }
}
