use crate::{
    AsyncTransformer,
    protocol::websocket::{WsError, WsMessage, WsSink, is_websocket_disconnected},
    socket::{Message, MessageAdminWs, reconnecting::SocketUpdate},
};
use futures::{Sink, SinkExt};
use pin_project::pin_project;
use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use tracing::{debug, info, warn};

#[pin_project]
pub struct ReconnectingSink<S> {
    #[pin]
    sink: Option<S>,
}

pub enum SinkCommand<Item> {
    Flush,
    Reconnect,
    Send(Item),
}

pub enum ReconnectingSinkError<E> {
    Reconnecting,
    Error(E),
}

impl AsyncTransformer for ReconnectingSink<WsSink> {
    type Input = Message<SocketUpdate<WsSink, MessageAdminWs>, SinkCommand<WsMessage>>;
    type Output = Result<(), ReconnectingSinkError<WsError>>;

    async fn transform(&mut self, input: Self::Input) -> Self::Output {
        use Message::*;
        use MessageAdminWs::*;

        match input {
            Admin(SocketUpdate::Connected(sink)) => {
                info!(socket_id = "todo", "todo");
                if let Some(mut old) = self.swap_sink(sink) {
                    old.close().await.map_err(ReconnectingSinkError::Error)?;
                }
            }
            Admin(SocketUpdate::Reconnecting) => {
                info!(socket_id = "todo", "todo");
                if let Some(mut old) = self.take_sink() {
                    old.close().await.map_err(ReconnectingSinkError::Error)?;
                }
            }
            Admin(SocketUpdate::Item(Ping(payload))) => {
                debug!(socket_id = "todo", ?payload, "received WebSocket Ping");
            }
            Admin(SocketUpdate::Item(Pong(payload))) => {
                debug!(socket_id = "todo", ?payload, "received WebSocket Pong");
            }
            Admin(SocketUpdate::Item(Close(payload))) => {
                warn!(
                    socket_id = "todo",
                    ?payload,
                    "received WebSocket CloseFrame"
                );
            }
            Admin(SocketUpdate::Item(Error(error))) => {
                if is_websocket_disconnected(&error) {
                    warn!(socket_id = "todo", "todo");
                    if let Some(mut old) = self.take_sink() {
                        old.close().await.map_err(ReconnectingSinkError::Error)?;
                    }
                } else {
                    warn!(socket_id = "todo", "todo");
                }
            }
            Payload(SinkCommand::Flush) => {
                self.flush().await?;
            }
            Payload(SinkCommand::Reconnect) => {
                self.close().await?;
            }
            Payload(SinkCommand::Send(item)) => {
                self.send(item).await?;
            }
        }

        Ok(())
    }
}

impl<S> ReconnectingSink<S> {
    pub fn new(sink: Option<S>) -> Self {
        Self { sink }
    }

    pub fn swap_sink(&mut self, new_sink: S) -> Option<S> {
        self.sink.replace(new_sink)
    }

    pub fn take_sink(&mut self) -> Option<S> {
        self.sink.take()
    }
}

impl<S, Item> Sink<Item> for ReconnectingSink<S>
where
    S: Sink<Item>,
{
    type Error = ReconnectingSinkError<S::Error>;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();

        // Todo: This method returns Poll::Ready once the underlying sink is ready to receive data.
        //  If this method returns Poll::Pending, the current task is registered to be notified
        //  (via cx.waker().wake_by_ref()) when poll_ready should be called again.

        match this.sink.as_pin_mut() {
            Some(sink) => sink.poll_ready(cx).map_err(ReconnectingSinkError::Error),
            None => Poll::Ready(Err(ReconnectingSinkError::Reconnecting)),
        }
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let this = self.project();

        match this.sink.as_pin_mut() {
            Some(sink) => sink.start_send(item).map_err(ReconnectingSinkError::Error),
            None => Err(ReconnectingSinkError::Reconnecting),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();

        match this.sink.as_pin_mut() {
            Some(sink) => sink.poll_flush(cx).map_err(ReconnectingSinkError::Error),
            None => Poll::Ready(Err(ReconnectingSinkError::Reconnecting)),
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();

        // Todo: If this function encounters an error, the sink should be considered to have failed
        //   permanently, and no more Sink methods should be called.

        match this.sink.as_pin_mut() {
            Some(sink) => sink.poll_close(cx).map_err(ReconnectingSinkError::Error),
            None => Poll::Ready(Ok(())), // Already closed/disconnected
        }
    }
}
