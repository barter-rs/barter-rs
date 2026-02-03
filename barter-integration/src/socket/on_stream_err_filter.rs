use crate::socket::on_stream_err::{OnStreamErr, StreamErrorHandler};
use futures::{Sink, Stream, ready};
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// Stream wrapper that applies error handling before filtering them out.
///
/// Wraps [`OnStreamErr`] and filters out all `Err` values that have
/// `StreamErrorAction::Continue`. Only `Ok` values are emitted.
#[derive(Debug)]
#[pin_project]
pub struct OnStreamErrFilter<S, ErrHandler> {
    #[pin]
    socket: OnStreamErr<S, ErrHandler>,
}

impl<S, ErrHandler> OnStreamErrFilter<S, ErrHandler> {
    pub fn new(socket: S, on_err: ErrHandler) -> Self {
        Self {
            socket: OnStreamErr::new(socket, on_err),
        }
    }
}

impl<S, StOk, StErr, ErrHandler> Stream for OnStreamErrFilter<S, ErrHandler>
where
    S: Stream<Item = Result<StOk, StErr>>,
    ErrHandler: StreamErrorHandler<StErr>,
{
    type Item = StOk;

    #[inline]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            let next_ready = ready!(this.socket.as_mut().poll_next(cx));

            let Some(result) = next_ready else {
                return Poll::Ready(None);
            };

            match result {
                Ok(item) => return Poll::Ready(Some(item)),
                Err(_) => continue,
            }
        }
    }
}

impl<St, ErrHandler, Item> Sink<Item> for OnStreamErrFilter<St, ErrHandler>
where
    St: Sink<Item>,
{
    type Error = St::Error;

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
    use crate::socket::on_stream_err::StreamErrorAction;
    use futures::StreamExt;
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use tokio_test::{assert_pending, assert_ready_eq};

    type TestError = &'static str;

    #[tokio::test]
    async fn test_on_stream_err_filter_yields_only_ok() {
        let waker = futures::task::noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        let (tx, rx) = mpsc::unbounded_channel::<Result<i32, TestError>>();
        let rx = UnboundedReceiverStream::new(rx);

        let mut stream =
            OnStreamErrFilter::new(rx, |_error: &TestError| StreamErrorAction::Continue);

        assert_pending!(stream.poll_next_unpin(&mut cx));

        tx.send(Ok(1)).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(1));

        tx.send(Err("error1")).unwrap();
        assert_pending!(stream.poll_next_unpin(&mut cx));

        tx.send(Ok(2)).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(2));

        tx.send(Err("error2")).unwrap();
        tx.send(Err("error3")).unwrap();
        assert_pending!(stream.poll_next_unpin(&mut cx));

        tx.send(Ok(3)).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(3));

        drop(tx);
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), None);
    }

    #[tokio::test]
    async fn test_on_stream_err_filter_reconnect_on_fatal() {
        let waker = futures::task::noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        let (tx, rx) = mpsc::unbounded_channel::<Result<i32, TestError>>();
        let rx = UnboundedReceiverStream::new(rx);

        let mut stream = OnStreamErrFilter::new(rx, |error: &TestError| {
            if *error == "fatal" {
                StreamErrorAction::Reconnect
            } else {
                StreamErrorAction::Continue
            }
        });

        tx.send(Ok(1)).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(1));

        tx.send(Err("non-fatal")).unwrap();
        assert_pending!(stream.poll_next_unpin(&mut cx));

        tx.send(Ok(2)).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(2));

        tx.send(Err("fatal")).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), None);
    }

    #[tokio::test]
    async fn test_on_stream_err_filter_all_errors() {
        let waker = futures::task::noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        let (tx, rx) = mpsc::unbounded_channel::<Result<i32, TestError>>();
        let rx = UnboundedReceiverStream::new(rx);

        let mut stream =
            OnStreamErrFilter::new(rx, |_error: &TestError| StreamErrorAction::Continue);

        tx.send(Err("error1")).unwrap();
        tx.send(Err("error2")).unwrap();
        tx.send(Err("error3")).unwrap();

        assert_pending!(stream.poll_next_unpin(&mut cx));

        drop(tx);
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), None);
    }
}
