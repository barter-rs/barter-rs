use crate::socket::{
    backoff::ReconnectBackoff,
    on_connect_err::{ConnectError, ConnectErrorAction, ConnectErrorHandler, ConnectErrorKind},
    on_stream_err::{StreamErrorAction, StreamErrorHandler},
    sink::ReconnectingSink,
};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

// Todo: consider adding a with_stream_timeout for "next event timeout",
//       or maybe .timeout() is fine, but needs to be before stream_of_streams.flatten()
//
// Todo: Add method to .flatten() without the ConnectionUpdates
pub trait ReconnectingSocket
where
    Self: Stream,
{
    fn on_connect_err<Socket, ErrConnect, ErrHandler>(
        self,
        on_err: ErrHandler,
    ) -> impl Stream<Item = Socket>
    where
        Self: Stream<Item = Result<Socket, ConnectError<ErrConnect>>> + Sized,
        ErrHandler: ConnectErrorHandler<ErrConnect>,
    {
        self.scan(on_err, |on_err, result| {
            std::future::ready(match result {
                Ok(socket) => Some(Some(socket)),
                Err(error) => match on_err.handle(&error) {
                    ConnectErrorAction::Reconnect => Some(None),
                    ConnectErrorAction::Terminate => None,
                },
            })
        })
        .filter_map(std::future::ready)
    }

    fn on_stream_err<Sink, St, StOk, StErr, ErrHandler>(
        self,
        on_err: ErrHandler,
    ) -> impl Stream<Item = (Sink, impl Stream<Item = Result<StOk, StErr>>)>
    where
        Self: Stream<Item = (Sink, St)> + Sized,
        St: Stream<Item = Result<StOk, StErr>>,
        ErrHandler: StreamErrorHandler<StErr> + Clone + 'static,
    {
        self.map(move |(sink, stream)| {
            let mut on_err = on_err.clone();
            let stream = tokio_stream::StreamExt::map_while(stream, move |result| match result {
                Ok(event) => Some(Ok(event)),
                Err(error) => match on_err.handle(&error) {
                    StreamErrorAction::Continue => Some(Err(error)),
                    StreamErrorAction::Reconnect => None,
                },
            });
            (sink, stream)
        })
    }

    fn on_stream_err_filter<Sink, St, StOk, StErr, ErrHandler>(
        self,
        on_err: ErrHandler,
    ) -> impl Stream<Item = (Sink, impl Stream<Item = StOk>)>
    where
        Self: Stream<Item = (Sink, St)> + Sized,
        St: Stream<Item = Result<StOk, StErr>>,
        ErrHandler: StreamErrorHandler<StErr> + Clone + 'static,
    {
        self.on_stream_err(on_err).map(|(sink, stream)| {
            let stream = stream.filter_map(|result| std::future::ready(result.ok()));
            (sink, stream)
        })
    }

    fn into_reconnecting_sink_and_stream<Origin, Sink, St>(
        self,
        origin: Origin,
    ) -> (
        ReconnectingSink<Sink>,
        impl Stream<Item = StreamEvent<Origin, St::Item>>,
    )
    where
        Self: Stream<Item = (Sink, St)> + Sized,
        Origin: Clone + 'static,
        St: Stream,
    {
        let (sink_tx, sink_rx) = tokio::sync::watch::channel(None);

        let stream = self
            .with_connection_updates(origin)
            .map(move |event| match event {
                SocketEvent::Connected(origin, sink) => {
                    let _ = sink_tx.send(Some(sink));
                    StreamEvent::Reconnecting(origin)
                }
                SocketEvent::Reconnecting(origin) => {
                    let _ = sink_tx.send(None);
                    StreamEvent::Reconnecting(origin)
                }
                SocketEvent::Item(item) => StreamEvent::Item(item),
            });

        (ReconnectingSink::new(sink_rx), stream)
    }

    fn with_connection_updates<Origin, Sink, St>(
        self,
        origin: Origin,
    ) -> impl Stream<Item = SocketEvent<Origin, Sink, St::Item>>
    where
        Self: Stream<Item = (Sink, St)> + Sized,
        St: Stream,
        Origin: Clone + 'static,
    {
        self.map(move |(sink, stream)| {
            futures::stream::once(std::future::ready(SocketEvent::Connected(
                origin.clone(),
                sink,
            )))
            .chain(stream.map(SocketEvent::Item).chain(futures::stream::once(
                std::future::ready(SocketEvent::Reconnecting(origin.clone())),
            )))
        })
        .flatten()
    }

    fn route_sinks<Origin, Sink, T, FnRoute, FnRouteErr>(
        self,
        route: FnRoute,
    ) -> impl Stream<Item = StreamEvent<Origin, T>>
    where
        Self: Stream<Item = SocketEvent<Origin, Sink, T>> + Unpin + Sized,
        FnRoute: AsyncFnMut(Origin, Sink) -> Result<(), FnRouteErr>,
    {
        futures::stream::unfold((self, route), |(mut stream, mut route)| async move {
            let event = stream.next().await?;
            match event {
                SocketEvent::Connected(origin, sink) => {
                    if route(origin, sink).await.is_err() {
                        None
                    } else {
                        Some((None, (stream, route)))
                    }
                }
                SocketEvent::Reconnecting(origin) => {
                    Some((Some(StreamEvent::Reconnecting(origin)), (stream, route)))
                }
                SocketEvent::Item(item) => Some((Some(StreamEvent::Item(item)), (stream, route))),
            }
        })
        .filter_map(std::future::ready)
    }
}

impl<St> ReconnectingSocket for St where St: Stream {}

pub fn init_reconnecting_socket<FnConnect, Backoff, Socket, ErrConnect>(
    connect: FnConnect,
    timeout_connect: std::time::Duration,
    backoff: Backoff,
) -> impl Stream<Item = Result<Socket, ConnectError<ErrConnect>>>
where
    FnConnect: AsyncFnMut() -> Result<Socket, ErrConnect>,
    Backoff: ReconnectBackoff,
{
    struct State<F, B> {
        connect: F,
        backoff: B,
        reconnection_attempt: u32,
    }

    futures::stream::unfold(
        State {
            connect,
            backoff,
            reconnection_attempt: 0,
        },
        move |mut state| async move {
            // Apply reconnection backoff
            let backoff = state.backoff.reconnect_backoff(state.reconnection_attempt);
            tokio::time::sleep(backoff).await;

            // Connect with timeout
            let result = match tokio::time::timeout(timeout_connect, (state.connect)()).await {
                Ok(Ok(socket)) => {
                    state.reconnection_attempt = 0;
                    Ok(socket)
                }
                Ok(Err(error)) => {
                    state.reconnection_attempt = state.reconnection_attempt.saturating_add(1);
                    Err(ConnectError {
                        reconnection_attempt: state.reconnection_attempt,
                        kind: ConnectErrorKind::Connect(error),
                    })
                }
                Err(_elapsed) => {
                    state.reconnection_attempt = state.reconnection_attempt.saturating_add(1);
                    Err(ConnectError {
                        reconnection_attempt: state.reconnection_attempt,
                        kind: ConnectErrorKind::Timeout,
                    })
                }
            };

            Some((result, state))
        },
    )
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum SocketEvent<Origin, Sink, T> {
    Connected(Origin, Sink),
    Reconnecting(Origin),
    Item(T),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum StreamEvent<Origin, T> {
    Reconnecting(Origin),
    Item(T),
}

impl<Origin, T> From<T> for StreamEvent<Origin, T> {
    fn from(value: T) -> Self {
        Self::Item(value)
    }
}

impl<Origin, T> StreamEvent<Origin, T> {
    pub fn map<F, O>(self, op: F) -> Event<Origin, O>
    where
        F: FnOnce(T) -> O,
    {
        match self {
            StreamEvent::Reconnecting(origin) => StreamEvent::Reconnecting(origin),
            StreamEvent::Item(item) => StreamEvent::Item(op(item)),
        }
    }
}

impl<Origin, T, E> StreamEvent<Origin, Result<T, E>> {
    pub fn map_ok<F, O>(self, op: F) -> StreamEvent<Origin, Result<O, E>>
    where
        F: FnOnce(T) -> O,
    {
        match self {
            StreamEvent::Reconnecting(origin) => StreamEvent::Reconnecting(origin),
            StreamEvent::Item(result) => StreamEvent::Item(result.map(op)),
        }
    }

    pub fn map_err<F, O>(self, op: F) -> StreamEvent<Origin, Result<T, O>>
    where
        F: FnOnce(E) -> O,
    {
        match self {
            StreamEvent::Reconnecting(origin) => StreamEvent::Reconnecting(origin),
            StreamEvent::Item(result) => StreamEvent::Item(result.map_err(op)),
        }
    }
}
