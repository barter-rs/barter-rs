use futures::{Sink, Stream, ready};
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// Stream adapter that forwards clones of matching items whilst also yielding all items.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::ext::BarterStreamExt;
    use futures::StreamExt;
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use tokio_test::{assert_pending, assert_ready_eq};

    #[tokio::test]
    async fn test_forward_clone_by() {
        let waker = futures::task::noop_waker_ref();
        let mut cx = std::task::Context::from_waker(waker);

        let (tx, rx) = mpsc::unbounded_channel::<i32>();
        let rx = UnboundedReceiverStream::new(rx);

        let (forward_tx, mut forward_rx) = mpsc::unbounded_channel::<i32>();

        let mut stream = rx.forward_clone_by(
            |item| *item % 2 == 0,
            move |item| forward_tx.send(item).map_err(|_| ()),
        );

        assert_pending!(stream.poll_next_unpin(&mut cx));

        tx.send(1).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(1));
        assert!(forward_rx.try_recv().is_err());

        tx.send(2).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(2));
        assert_eq!(forward_rx.try_recv().unwrap(), 2);

        tx.send(3).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(3));
        assert!(forward_rx.try_recv().is_err());

        tx.send(4).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(4));
        assert_eq!(forward_rx.try_recv().unwrap(), 4);

        drop(tx);
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), None);
    }

    #[tokio::test]
    async fn test_forward_clone_by_terminates_on_forward_error() {
        let waker = futures::task::noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        let (tx, rx) = mpsc::unbounded_channel::<i32>();
        let rx = UnboundedReceiverStream::new(rx);

        let mut stream = rx.forward_clone_by(
            |item| *item % 2 == 0,
            |item| if item == 4 { Err(()) } else { Ok(()) },
        );

        tx.send(1).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(1));

        tx.send(2).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(2));

        tx.send(4).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), None);
    }
}
