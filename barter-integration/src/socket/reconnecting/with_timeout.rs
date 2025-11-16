use futures::Stream;
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

/// Stream adapter that terminates the stream if no item is received within a timeout duration.
///
/// When the timeout elapses while waiting for the next item:
/// - Calls the timeout handler
/// - Ends the stream
#[pin_project]
pub struct WithTimeout<S, TimeoutHandler> {
    #[pin]
    socket: tokio_stream::Timeout<S>,
    handler: TimeoutHandler,
}

impl<S, TimeoutHandler> WithTimeout<S, TimeoutHandler> {
    pub fn new(socket: S, timeout: std::time::Duration, handler: TimeoutHandler) -> Self
    where
        S: Stream,
    {
        Self {
            socket: tokio_stream::StreamExt::timeout(socket, timeout),
            handler,
        }
    }
}

impl<S, TimeoutHandler> Stream for WithTimeout<S, TimeoutHandler>
where
    S: Stream,
    TimeoutHandler: Fn(),
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        let next_ready = ready!(this.socket.poll_next(cx));

        let Some(timeout_result) = next_ready else {
            return Poll::Ready(None);
        };

        match timeout_result {
            Ok(item) => Poll::Ready(Some(item)),
            Err(_elapsed) => {
                (this.handler)();
                Poll::Ready(None)
            }
        }
    }
}

// impl<S, TimeoutHandler, Item> Sink<Item> for WithTimeout<S, TimeoutHandler>
// where
//     S: Sink<Item>,
// {
//     type Error = S::Error;
//
//     fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         self.project().socket.poll_ready(cx)
//     }
//
//     fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
//         self.project().socket.start_send(item)
//     }
//
//     fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         self.project().socket.poll_flush(cx)
//     }
//
//     fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         self.project().socket.poll_close(cx)
//     }
// }
