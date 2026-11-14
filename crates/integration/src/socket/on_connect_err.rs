use futures::{Sink, Stream};
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

/// Handles connection errors and determines the appropriate [`ConnectErrorAction`].
pub trait ConnectErrorHandler<Err> {
    /// Handles a connection error and returns the action to take.
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

/// Connection error with reconnection attempt count.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ConnectError<ErrConnect> {
    pub reconnection_attempt: u32,
    pub kind: ConnectErrorKind<ErrConnect>,
}

/// Connection error variants.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ConnectErrorKind<ErrConnect> {
    /// Connection attempt failed.
    Connect(ErrConnect),
    /// Connection attempt timed out.
    Timeout,
}

/// Action to take in response to a connection error.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ConnectErrorAction {
    /// Attempt to reconnect.
    Reconnect,
    /// Terminate the stream.
    Terminate,
}

/// Stream adapter that handles connection errors using a custom error handler.
///
/// - Ok(socket) items are passed through to the output stream
/// - Err(error) items are handled by the error handler:
///   - ConnectErrorAction::Reconnect: Filter out the error and continue polling
///   - ConnectErrorAction::Terminate: End the stream
#[derive(Debug)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::socket::ReconnectingSocket;
    use futures::StreamExt;
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use tokio_test::{assert_pending, assert_ready_eq};

    type TestSocket = i32;
    type TestError = &'static str;

    #[tokio::test]
    async fn test_on_connect_err_passes_through_success() {
        let waker = futures::task::noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        let (tx, rx) = mpsc::unbounded_channel::<Result<TestSocket, ConnectError<TestError>>>();
        let rx = UnboundedReceiverStream::new(rx);

        let mut stream =
            rx.on_connect_err(|_error: &ConnectError<TestError>| ConnectErrorAction::Reconnect);

        assert_pending!(stream.poll_next_unpin(&mut cx));

        tx.send(Ok(1)).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(1));

        tx.send(Ok(2)).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(2));

        drop(tx);
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), None);
    }

    #[tokio::test]
    async fn test_on_connect_err_reconnect_action() {
        let waker = futures::task::noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        let (tx, rx) = mpsc::unbounded_channel::<Result<TestSocket, ConnectError<TestError>>>();
        let rx = UnboundedReceiverStream::new(rx);

        let mut stream =
            rx.on_connect_err(|_error: &ConnectError<TestError>| ConnectErrorAction::Reconnect);

        tx.send(Err(ConnectError {
            reconnection_attempt: 1,
            kind: ConnectErrorKind::Connect("network error"),
        }))
        .unwrap();
        assert_pending!(stream.poll_next_unpin(&mut cx));

        tx.send(Err(ConnectError {
            reconnection_attempt: 2,
            kind: ConnectErrorKind::Timeout,
        }))
        .unwrap();
        assert_pending!(stream.poll_next_unpin(&mut cx));

        tx.send(Ok(42)).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(42));

        drop(tx);
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), None);
    }

    #[tokio::test]
    async fn test_on_connect_err_terminate_action() {
        let waker = futures::task::noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        let (tx, rx) = mpsc::unbounded_channel::<Result<TestSocket, ConnectError<TestError>>>();
        let rx = UnboundedReceiverStream::new(rx);

        let mut stream = rx.on_connect_err(|error: &ConnectError<TestError>| {
            if error.reconnection_attempt >= 3 {
                ConnectErrorAction::Terminate
            } else {
                ConnectErrorAction::Reconnect
            }
        });

        tx.send(Ok(1)).unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some(1));

        tx.send(Err(ConnectError {
            reconnection_attempt: 1,
            kind: ConnectErrorKind::Connect("error"),
        }))
        .unwrap();
        assert_pending!(stream.poll_next_unpin(&mut cx));

        tx.send(Err(ConnectError {
            reconnection_attempt: 3,
            kind: ConnectErrorKind::Connect("error"),
        }))
        .unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), None);
    }
}
