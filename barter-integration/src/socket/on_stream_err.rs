use futures::{Sink, Stream};
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

/// Handles stream errors and determines the appropriate [`StreamErrorAction`].
pub trait StreamErrorHandler<Err> {
    /// Handles a stream error and returns the action to take.
    fn handle(&mut self, error: &Err) -> StreamErrorAction;
}

impl<Err, F> StreamErrorHandler<Err> for F
where
    F: FnMut(&Err) -> StreamErrorAction,
{
    #[inline]
    fn handle(&mut self, error: &Err) -> StreamErrorAction {
        self(error)
    }
}

/// Action to take in response to a stream error.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StreamErrorAction {
    /// Keep the stream alive.
    Continue,
    /// End the stream and trigger reconnection.
    Reconnect,
}

/// Stream wrapper that applies error handling to a Result stream.
///
/// When an error occurs:
/// - `StreamErrorAction::Continue`: Pass the error through
/// - `StreamErrorAction::Reconnect`: End the stream (triggers reconnection)
#[derive(Debug)]
#[pin_project]
pub struct OnStreamErr<S, ErrHandler> {
    #[pin]
    socket: S,
    on_err: ErrHandler,
}

impl<S, ErrHandler> OnStreamErr<S, ErrHandler> {
    pub fn new(socket: S, on_err: ErrHandler) -> Self {
        Self { socket, on_err }
    }
}

impl<S, StOk, StErr, ErrHandler> Stream for OnStreamErr<S, ErrHandler>
where
    S: Stream<Item = Result<StOk, StErr>>,
    ErrHandler: StreamErrorHandler<StErr>,
{
    type Item = Result<StOk, StErr>;

    #[inline]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        let next_ready = ready!(this.socket.as_mut().poll_next(cx));

        let Some(result) = next_ready else {
            return Poll::Ready(None);
        };

        match result {
            Ok(item) => Poll::Ready(Some(Ok(item))),
            Err(error) => match (this.on_err).handle(&error) {
                StreamErrorAction::Continue => Poll::Ready(Some(Err(error))),
                StreamErrorAction::Reconnect => Poll::Ready(None),
            },
        }
    }
}

impl<St, ErrHandler, Item> Sink<Item> for OnStreamErr<St, ErrHandler>
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
    use futures::StreamExt;
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use tokio_test::{assert_pending, assert_ready};

    type TestError = &'static str;

    #[tokio::test]
    async fn test_on_stream_err_passes_through_ok() {
        let waker = futures::task::noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        let (tx, rx) = mpsc::unbounded_channel::<Result<i32, TestError>>();
        let rx = UnboundedReceiverStream::new(rx);

        let mut stream = OnStreamErr::new(rx, |_error: &TestError| StreamErrorAction::Continue);

        assert_pending!(stream.poll_next_unpin(&mut cx));

        tx.send(Ok(1)).unwrap();
        assert_eq!(assert_ready!(stream.poll_next_unpin(&mut cx)), Some(Ok(1)));

        tx.send(Ok(2)).unwrap();
        assert_eq!(assert_ready!(stream.poll_next_unpin(&mut cx)), Some(Ok(2)));

        drop(tx);
        assert_eq!(assert_ready!(stream.poll_next_unpin(&mut cx)), None);
    }

    #[tokio::test]
    async fn test_on_stream_err_continue_action() {
        let waker = futures::task::noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        let (tx, rx) = mpsc::unbounded_channel::<Result<i32, TestError>>();
        let rx = UnboundedReceiverStream::new(rx);

        let mut stream = OnStreamErr::new(rx, |_error: &TestError| StreamErrorAction::Continue);

        tx.send(Ok(1)).unwrap();
        assert_eq!(assert_ready!(stream.poll_next_unpin(&mut cx)), Some(Ok(1)));

        tx.send(Err("error1")).unwrap();
        assert_eq!(
            assert_ready!(stream.poll_next_unpin(&mut cx)),
            Some(Err("error1"))
        );

        tx.send(Ok(2)).unwrap();
        assert_eq!(assert_ready!(stream.poll_next_unpin(&mut cx)), Some(Ok(2)));

        drop(tx);
        assert_eq!(assert_ready!(stream.poll_next_unpin(&mut cx)), None);
    }

    #[tokio::test]
    async fn test_on_stream_err_reconnect_action() {
        let waker = futures::task::noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        let (tx, rx) = mpsc::unbounded_channel::<Result<i32, TestError>>();
        let rx = UnboundedReceiverStream::new(rx);

        let mut stream = OnStreamErr::new(rx, |error: &TestError| {
            if *error == "fatal" {
                StreamErrorAction::Reconnect
            } else {
                StreamErrorAction::Continue
            }
        });

        tx.send(Ok(1)).unwrap();
        assert_eq!(assert_ready!(stream.poll_next_unpin(&mut cx)), Some(Ok(1)));

        tx.send(Err("non-fatal")).unwrap();
        assert_eq!(
            assert_ready!(stream.poll_next_unpin(&mut cx)),
            Some(Err("non-fatal"))
        );

        tx.send(Err("fatal")).unwrap();
        assert_eq!(assert_ready!(stream.poll_next_unpin(&mut cx)), None);
    }
}
