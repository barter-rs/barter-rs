use crate::socket::reconnecting::{
    backoff::ReconnectBackoff,
    forward_by::ForwardBy,
    on_connect_err::{ConnectError, ConnectErrorHandler, ConnectErrorKind, OnConnectErr},
    on_stream_err::{OnStreamErr, StreamErrorHandler},
    on_stream_err_filter::OnStreamErrFilter,
};
use futures::{Sink, Stream, StreamExt, stream::SplitSink};
use serde::{Deserialize, Serialize};

pub mod backoff;
pub mod forward_by;
pub mod on_connect_err;
pub mod on_stream_err;
pub mod on_stream_err_filter;
pub mod sink;
pub mod with_timeout;

// Todo:
//  - Add ability to .flatten() without ConnectionUpdates
//  - Do I want ReconnectingStream as well as for Stream specific combinators?
pub trait ReconnectingSocket
where
    Self: Stream,
{
    fn with_timeout<TimeoutHandler>(
        self,
        timeout_next_item: std::time::Duration,
        on_timeout: TimeoutHandler,
    ) -> impl Stream<Item = Self::Item>
    where
        Self: Stream + Sized,
        TimeoutHandler: Fn() + 'static,
    {
        use tokio_stream::StreamExt;
        self.timeout(timeout_next_item)
            .map_while(move |timeout_result| match timeout_result {
                Ok(item) => Some(item),
                Err(_elapsed) => {
                    on_timeout();
                    None
                }
            })
    }

    fn on_connect_err<Socket, ErrConnect, ErrHandler>(
        self,
        on_err: ErrHandler,
    ) -> OnConnectErr<Self, ErrHandler>
    where
        Self: Stream<Item = Result<Socket, ConnectError<ErrConnect>>> + Sized,
        ErrHandler: ConnectErrorHandler<ErrConnect>,
    {
        OnConnectErr::new(self, on_err)
    }

    fn on_stream_err<Socket, StOk, StErr, ErrHandler>(
        self,
        on_err: ErrHandler,
    ) -> impl Stream<Item = OnStreamErr<Socket, ErrHandler>>
    where
        Self: Stream<Item = Socket> + Sized,
        Socket: Stream<Item = Result<StOk, StErr>>,
        ErrHandler: StreamErrorHandler<StErr> + Clone + 'static,
    {
        self.map(move |socket| OnStreamErr::new(socket, on_err.clone()))
    }

    fn on_stream_err_filter<Socket, StOk, StErr, ErrHandler>(
        self,
        on_err: ErrHandler,
    ) -> impl Stream<Item = OnStreamErrFilter<Socket, ErrHandler>>
    where
        Self: Stream<Item = Socket> + Sized,
        Socket: Stream<Item = Result<StOk, StErr>>,
        ErrHandler: StreamErrorHandler<StErr> + Clone + 'static,
    {
        self.map(move |socket| OnStreamErrFilter::new(socket, on_err.clone()))
    }

    // Todo: What to do if need to Forward as well as Keep? eg/ OrderResponse, maybe Audit stream is enough?
    fn forward_by<A, B, FnPredicate, FnForward>(
        self,
        predicate: FnPredicate,
        forward: FnForward,
    ) -> ForwardBy<Self, FnPredicate, FnForward>
    where
        Self: Stream + Sized,
        FnPredicate: Fn(Self::Item) -> futures::future::Either<A, B>,
        FnForward: FnMut(A) -> Result<(), ()>,
    {
        ForwardBy::new(self, predicate, forward)
    }

    fn with_socket_updates<Socket, SinkItem>(
        self,
    ) -> impl Stream<Item = SocketUpdate<SplitSink<Socket, SinkItem>, Socket::Item>>
    where
        Self: Stream<Item = Socket> + Sized,
        Socket: Sink<SinkItem> + Stream,
    {
        use futures::stream::once;
        use std::future::ready;

        self.map(move |socket| {
            let (sink, stream) = socket.split();
            once(ready(SocketUpdate::Connected(sink))).chain(
                stream
                    .map(SocketUpdate::Item)
                    .chain(once(ready(SocketUpdate::Reconnecting))),
            )
        })
        .flatten()
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
pub enum SocketUpdate<Sink, T> {
    Connected(Sink),
    Reconnecting,
    Item(T),
}

impl<Sink, T> From<T> for SocketUpdate<Sink, T> {
    fn from(value: T) -> Self {
        Self::Item(value)
    }
}

impl<Sink, T> SocketUpdate<Sink, T> {
    pub fn map<F, O>(self, op: F) -> SocketUpdate<Sink, O>
    where
        F: FnOnce(T) -> O,
    {
        match self {
            SocketUpdate::Connected(sink) => SocketUpdate::Connected(sink),
            SocketUpdate::Reconnecting => SocketUpdate::Reconnecting,
            SocketUpdate::Item(item) => SocketUpdate::Item(op(item)),
        }
    }
}

impl<Sink, T, E> SocketUpdate<Sink, Result<T, E>> {
    pub fn map_ok<F, O>(self, op: F) -> SocketUpdate<Sink, Result<O, E>>
    where
        F: FnOnce(T) -> O,
    {
        match self {
            SocketUpdate::Connected(sink) => SocketUpdate::Connected(sink),
            SocketUpdate::Reconnecting => SocketUpdate::Reconnecting,
            SocketUpdate::Item(result) => SocketUpdate::Item(result.map(op)),
        }
    }

    pub fn map_err<F, O>(self, op: F) -> SocketUpdate<Sink, Result<T, O>>
    where
        F: FnOnce(E) -> O,
    {
        match self {
            SocketUpdate::Connected(sink) => SocketUpdate::Connected(sink),
            SocketUpdate::Reconnecting => SocketUpdate::Reconnecting,
            SocketUpdate::Item(result) => SocketUpdate::Item(result.map_err(op)),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum StreamUpdate<T> {
    Reconnecting,
    Item(T),
}

impl<T> From<T> for StreamUpdate<T> {
    fn from(value: T) -> Self {
        Self::Item(value)
    }
}

impl<T> StreamUpdate<T> {
    pub fn map<F, O>(self, op: F) -> StreamUpdate<O>
    where
        F: FnOnce(T) -> O,
    {
        match self {
            StreamUpdate::Reconnecting => StreamUpdate::Reconnecting,
            StreamUpdate::Item(item) => StreamUpdate::Item(op(item)),
        }
    }
}

impl<T, E> StreamUpdate<Result<T, E>> {
    pub fn map_ok<F, O>(self, op: F) -> StreamUpdate<Result<O, E>>
    where
        F: FnOnce(T) -> O,
    {
        match self {
            StreamUpdate::Reconnecting => StreamUpdate::Reconnecting,
            StreamUpdate::Item(result) => StreamUpdate::Item(result.map(op)),
        }
    }

    pub fn map_err<F, O>(self, op: F) -> StreamUpdate<Result<T, O>>
    where
        F: FnOnce(E) -> O,
    {
        match self {
            StreamUpdate::Reconnecting => StreamUpdate::Reconnecting,
            StreamUpdate::Item(result) => StreamUpdate::Item(result.map_err(op)),
        }
    }
}
