use futures::{Sink, Stream};
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

pub trait StreamErrorHandler<Err> {
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StreamErrorAction {
    Continue,
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
