use crate::socket::reconnecting::on_stream_err::{OnStreamErr, StreamErrorHandler};
use futures::{Sink, Stream, ready};
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// Stream wrapper that applies error handling and filters out errors.
///
/// Wraps `OnStreamErr` and filters out all `Err` values that have
/// `StreamErrorAction::Continue`. Only `Ok` values are emitted.
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
