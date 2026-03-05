use crate::socket::{
    backoff::ReconnectBackoff,
    on_connect_err::{ConnectError, ConnectErrorHandler, ConnectErrorKind, OnConnectErr},
    on_stream_err::{OnStreamErr, StreamErrorHandler},
    on_stream_err_filter::OnStreamErrFilter,
    update::SocketUpdate,
};
use futures::{Sink, Stream, stream::SplitSink};

/// Backoff strategies for reconnection attempts.
pub mod backoff;

/// Connection error handling.
pub mod on_connect_err;

/// Stream error handling.
pub mod on_stream_err;

/// Stream error handling with filtering.
pub mod on_stream_err_filter;

/// Defines the socket lifecycle [`SocketUpdate`] event.
pub mod update;

/// Extension trait providing reconnection utilities for streams.
pub trait ReconnectingSocket
where
    Self: Stream,
{
    /// Handles connection errors using the provided [`ConnectErrorHandler`].
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

    /// Applies error handling to the inner Stream using the provided [`StreamErrorHandler`].
    ///
    /// Errors may be passed through or trigger a reconnecting.
    fn on_stream_err<Socket, StOk, StErr, ErrHandler>(
        self,
        on_err: ErrHandler,
    ) -> impl Stream<Item = OnStreamErr<Socket, ErrHandler>>
    where
        Self: Stream<Item = Socket> + Sized,
        Socket: Stream<Item = Result<StOk, StErr>>,
        ErrHandler: StreamErrorHandler<StErr> + Clone + 'static,
    {
        use futures::StreamExt;
        self.map(move |socket| OnStreamErr::new(socket, on_err.clone()))
    }

    /// Similar to [`ReconnectingSocket::on_stream_err`] but filters all errors after applying
    /// the provided [`StreamErrorHandler`].
    fn on_stream_err_filter<Socket, StOk, StErr, ErrHandler>(
        self,
        on_err: ErrHandler,
    ) -> impl Stream<Item = OnStreamErrFilter<Socket, ErrHandler>>
    where
        Self: Stream<Item = Socket> + Sized,
        Socket: Stream<Item = Result<StOk, StErr>>,
        ErrHandler: StreamErrorHandler<StErr> + Clone + 'static,
    {
        use futures::StreamExt;
        self.map(move |socket| OnStreamErrFilter::new(socket, on_err.clone()))
    }

    /// Wrap stream items with [`SocketUpdate`] lifecycle events.
    fn with_socket_updates<Socket, SinkItem>(
        self,
    ) -> impl Stream<Item = SocketUpdate<SplitSink<Socket, SinkItem>, Socket::Item>>
    where
        Self: Stream<Item = Socket> + Sized,
        Socket: Sink<SinkItem> + Stream,
    {
        use futures::{StreamExt, stream::once};
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

/// Initialises a "reconnecting socket" using the provided connect function.
///
/// Upon disconnecting, the [`ReconnectBackoff`] is used to determine how long to wait
/// between reconnecting attempts.
///
/// Returns a `Stream` of `Socket` connection results.
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
