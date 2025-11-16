use crate::socket::reconnecting::{
    backoff::ReconnectBackoff,
    forward_by::ForwardBy,
    on_connect_err::{ConnectError, ConnectErrorHandler, ConnectErrorKind, OnConnectErr},
    on_stream_err::{OnStreamErr, StreamErrorHandler},
    on_stream_err_filter::OnStreamErrFilter,
};
use futures::{Sink, Stream, StreamExt, stream::SplitSink};
use serde::{Deserialize, Serialize};
use sink::ReconnectingSink;

pub mod backoff;
pub mod forward_by;
pub mod on_connect_err;
pub mod on_stream_err;
pub mod on_stream_err_filter;
pub mod sink;
pub mod with_timeout;

// Todo: Add method to .flatten() without the ConnectionUpdates
pub trait ReconnectingSocket
where
    Self: Stream,
{
    // Todo: ReconnectingStream maybe?
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
    // Stream<Item = Socket>
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
    // Stream<Item = B>
    where
        Self: Stream + Sized,
        FnPredicate: Fn(Self::Item) -> futures::future::Either<A, B>,
        FnForward: FnMut(A) -> Result<(), ()>,
    {
        ForwardBy::new(self, predicate, forward)
    }

    // fn into_reconnecting_sink_and_stream<Origin, Sink, St>(
    //     self,
    //     origin: Origin,
    // ) -> (
    //     ReconnectingSink<Sink>,
    //     impl Stream<Item = StreamUpdate<St::Item, Origin>>,
    // )
    // where
    //     Self: Stream<Item = (Sink, St)> + Sized,
    //     Origin: Clone + 'static,
    //     St: Stream,
    // {
    //     let (sink_tx, sink_rx) = tokio::sync::watch::channel(None);
    //
    //     let stream = self
    //         .with_connection_updates(origin)
    //         .map(move |event| match event {
    //             SocketUpdate::Connected(origin, sink) => {
    //                 let _ = sink_tx.send(Some(sink));
    //                 StreamUpdate::Reconnecting(origin)
    //             }
    //             SocketUpdate::Reconnecting(origin) => {
    //                 let _ = sink_tx.send(None);
    //                 StreamUpdate::Reconnecting(origin)
    //             }
    //             SocketUpdate::Item(item) => StreamUpdate::Item(item),
    //         });
    //
    //     (ReconnectingSink::new(sink_rx), stream)
    // }

    fn with_socket_updates<Socket, SinkItem>(
        self,
    ) -> impl Stream<Item = SocketUpdate<SplitSink<Self, SinkItem>, Socket::Item>>
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

    // // ReconnectingSocket
    // fn with_connection_updates<Origin, Sink, St>(
    //     self,
    //     origin: Origin,
    // ) -> impl Stream<Item = SocketUpdate<Origin, Sink, St::Item>>
    // where
    //     Self: Stream<Item = (Sink, St)> + Sized,
    //     St: Stream,
    //     Origin: Clone + 'static,
    // {
    //     self.map(move |(sink, stream)| {
    //         futures::stream::once(std::future::ready(SocketUpdate::Connected(
    //             origin.clone(),
    //             sink,
    //         )))
    //         .chain(stream.map(SocketUpdate::Item).chain(futures::stream::once(
    //             std::future::ready(SocketUpdate::Reconnecting(origin.clone())),
    //         )))
    //     })
    //     .flatten()
    // }

    // fn route_sinks<Origin, Sink, T, FnRoute, FnRouteErr>(
    //     self,
    //     route: FnRoute,
    // ) -> impl Stream<Item = StreamUpdate<Origin, T>>
    // where
    //     Self: Stream<Item = SocketUpdate<Origin, Sink, T>> + Unpin + Sized,
    //     FnRoute: AsyncFnMut(Origin, Sink) -> Result<(), FnRouteErr>,
    // {
    //     futures::stream::unfold((self, route), |(mut stream, mut route)| async move {
    //         let event = stream.next().await?;
    //         match event {
    //             SocketUpdate::Connected(origin, sink) => {
    //                 if route(origin, sink).await.is_err() {
    //                     None
    //                 } else {
    //                     Some((None, (stream, route)))
    //                 }
    //             }
    //             SocketUpdate::Reconnecting(origin) => {
    //                 Some((Some(StreamUpdate::Reconnecting(origin)), (stream, route)))
    //             }
    //             SocketUpdate::Item(item) => Some((Some(StreamUpdate::Item(item)), (stream, route))),
    //         }
    //     })
    //     .filter_map(std::future::ready)
    // }
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum StreamUpdate<T> {
    Reconnecting,
    Item(T),
}

impl<Origin, T> From<T> for StreamUpdate<Origin, T> {
    fn from(value: T) -> Self {
        Self::Item(value)
    }
}

impl<Origin, T> StreamUpdate<Origin, T> {
    pub fn map<F, O>(self, op: F) -> StreamUpdate<Origin, O>
    where
        F: FnOnce(T) -> O,
    {
        match self {
            StreamUpdate::Reconnecting(origin) => StreamUpdate::Reconnecting(origin),
            StreamUpdate::Item(item) => StreamUpdate::Item(op(item)),
        }
    }
}

impl<Origin, T, E> StreamUpdate<Origin, Result<T, E>> {
    pub fn map_ok<F, O>(self, op: F) -> StreamUpdate<Origin, Result<O, E>>
    where
        F: FnOnce(T) -> O,
    {
        match self {
            StreamUpdate::Reconnecting(origin) => StreamUpdate::Reconnecting(origin),
            StreamUpdate::Item(result) => StreamUpdate::Item(result.map(op)),
        }
    }

    pub fn map_err<F, O>(self, op: F) -> StreamUpdate<Origin, Result<T, O>>
    where
        F: FnOnce(E) -> O,
    {
        match self {
            StreamUpdate::Reconnecting(origin) => StreamUpdate::Reconnecting(origin),
            StreamUpdate::Item(result) => StreamUpdate::Item(result.map_err(op)),
        }
    }
}
