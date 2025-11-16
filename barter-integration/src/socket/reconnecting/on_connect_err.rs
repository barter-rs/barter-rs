use futures::{Sink, Stream};
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

pub trait ConnectErrorHandler<Err> {
    fn handle(&mut self, error: &ConnectError<Err>) -> ConnectErrorAction;
}

impl<Err, F> ConnectErrorHandler<Err> for F
where
    F: FnMut(&ConnectError<Err>) -> ConnectErrorAction,
{
    #[inline]
    fn handle(&mut self, error: &ConnectError<Err>) -> ConnectErrorAction {
        self(error)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ConnectError<ErrConnect> {
    pub reconnection_attempt: u32,
    pub kind: ConnectErrorKind<ErrConnect>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ConnectErrorKind<ErrConnect> {
    Connect(ErrConnect),
    Timeout,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ConnectErrorAction {
    Reconnect,
    Terminate,
}

/// Stream adapter that handles connection errors using a custom error handler.
///
/// - Ok(socket) items are passed through to the output stream
/// - Err(error) items are handled by the error handler:
///   - ConnectErrorAction::Reconnect: Filter out the error and continue polling
///   - ConnectErrorAction::Terminate: End the stream
#[pin_project]
pub struct OnConnectErr<S, ErrHandler> {
    #[pin]
    socket: S,
    on_err: ErrHandler,
}

impl<S, ErrHandler> OnConnectErr<S, ErrHandler> {
    pub fn new(socket: S, on_err: ErrHandler) -> Self {
        Self { socket, on_err }
    }
}

impl<S, Socket, ErrConnect, ErrHandler> Stream for OnConnectErr<S, ErrHandler>
where
    S: Stream<Item = Result<Socket, ConnectError<ErrConnect>>>,
    ErrHandler: ConnectErrorHandler<ErrConnect>,
{
    type Item = Socket;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            let next_ready = ready!(this.socket.as_mut().poll_next(cx));

            let Some(result) = next_ready else {
                return Poll::Ready(None);
            };

            match result {
                Ok(socket) => {
                    return Poll::Ready(Some(socket));
                }
                Err(error) => {
                    match this.on_err.handle(&error) {
                        ConnectErrorAction::Reconnect => {
                            // Continue polling for the next item
                        }
                        ConnectErrorAction::Terminate => {
                            return Poll::Ready(None);
                        }
                    }
                }
            }
        }
    }
}

impl<S, ErrHandler, Item> Sink<Item> for OnConnectErr<S, ErrHandler>
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
