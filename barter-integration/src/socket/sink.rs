use futures::Sink;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub struct ReconnectingSink<Sink> {
    sink_rx: tokio::sync::watch::Receiver<Option<Sink>>,
}

pub enum ReconnectingSinkError<E> {
    Terminated,
    Reconnecting,
    Sink(E),
}

impl<S, Item> Sink<Item> for ReconnectingSink<S>
where
    S: Sink<Item>,
{
    type Error = ReconnectingSinkError<S::Error>;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let current = self.sink_rx.borrow();
        if let Some(sink) = current.as_ref() {
            Pin::new(sink)
                .poll_ready(cx)
                .map_err(ReconnectingSinkError::Sink)
        } else {
            Poll::Ready(Err(ReconnectingSinkError::Reconnecting))
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let mut current = self.sink_rx.borrow_mut();
        if let Some(sink) = current.as_mut() {
            Pin::new(sink)
                .start_send(item)
                .map_err(ReconnectingSinkError::Sink)
        } else {
            Err(ReconnectingSinkError::Reconnecting)
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }
}

impl<S> ReconnectingSink<S> {
    pub fn new(sink_rx: tokio::sync::watch::Receiver<Option<S>>) -> Self {
        Self { sink_rx }
    }

    pub fn is_connected(&self) -> bool {
        self.sink_rx.borrow().is_some()
    }

    pub fn is_terminated(&self) -> bool {
        self.sink_rx.has_changed().is_err()
    }

    pub async fn wait_for_reconnection<Item>(
        &mut self,
    ) -> Result<(), ReconnectingSinkError<S::Error>>
    where
        S: Sink<Item>,
    {
        while self.sink_rx.borrow().is_none() {
            self.sink_rx
                .changed()
                .await
                .map_err(|_| ReconnectingSinkError::Terminated)?;
        }
        Ok(())
    }
}
