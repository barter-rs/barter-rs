use derive_more::{Constructor, From};
use futures::Stream;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::future::Future;
use std::{convert, future};
use std::sync::Arc;
use tracing::{info, warn};
use crate::streams::consumer::StreamKey;

/// [`ReconnectingStream`] `Event` that communicates either `Stream::Item`, or that the inner
/// `Stream` is currently reconnecting.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum Event<Origin, T> {
    /// [`ReconnectingStream`] has disconnecting and is attempting to reconnect.
    Reconnecting(Origin),
    Item(T),
}

/// Utilities for handling a continually reconnecting [`Stream`] initialised via the
/// [`init_reconnecting_stream`] function.
pub trait ReconnectingStream
where
    Self: Stream + Sized,
{
    /// Add an exponential backoff policy to an initialised [`ReconnectingStream`] using the
    /// provided [`ReconnectionBackoffPolicy`].
    fn with_reconnect_backoff<St, InitError, Kind>(
        self,
        policy: ReconnectionBackoffPolicy,
        stream_key: StreamKey<Kind>,
    ) -> impl Stream<Item = St>
    where
        Self: Stream<Item = Result<St, InitError>>,
        St: Stream,
        InitError: Debug,
        Kind: Debug,
    {
        let stream_key = Arc::new(stream_key);
        self.enumerate()
            .scan(
                ReconnectionState::from(policy),
                move |state, (attempt, result)| match result {
                    Ok(stream) => {
                        info!(attempt, ?stream_key, "successfully initialised Stream");
                        state.reset_backoff();
                        futures::future::Either::Left(future::ready(Some(Ok(stream))))
                    }
                    Err(error) => {
                        warn!(
                            attempt,
                            ?stream_key,
                            ?error,
                            "failed to re-initialise Stream"
                        );
                        let sleep_fut = state.generate_sleep_future();
                        state.multiply_backoff();
                        futures::future::Either::Right(Box::pin(async move {
                            sleep_fut.await;
                            Some(Err(error))
                        }))
                    }
                },
            )
            .filter_map(|result| future::ready(result.ok()))
    }

    /// Maps every [`ReconnectingStream`] `Stream::Item` into an [`reconnect::Event::Item`](Event),
    /// and chain a [`reconnect::Event::Reconnecting`](Event)
    fn with_reconnection_events<St, Origin>(
        self,
        origin: Origin,
    ) -> impl Stream<Item = Event<Origin, St::Item>>
    where
        Self: Stream<Item = St>,
        St: Stream,
        Origin: Clone + 'static,
    {
        self.map(move |stream| {
            stream
                .map(Event::Item)
                .chain(futures::stream::once(future::ready(Event::Reconnecting(
                    origin.clone(),
                ))))
        })
        .flatten()
    }
}

impl<T> ReconnectingStream for T where T: Stream {}

/// Initialise a [`ReconnectingStream`] using the provided initialisation closure.
pub async fn init_reconnecting_stream<FnInit, St, FnInitError, FnInitFut>(
    init_stream: FnInit,
) -> Result<impl Stream<Item = Result<St, FnInitError>>, FnInitError>
where
    FnInit: Fn() -> FnInitFut,
    FnInitFut: Future<Output = Result<St, FnInitError>>,
{
    let initial = init_stream().await?;
    let reconnections = futures::stream::repeat_with(init_stream).then(convert::identity);

    Ok(futures::stream::once(future::ready(Ok(initial))).chain(reconnections))
}

/// Reconnection backoff policy for a [`ReconnectingStream::with_reconnect_backoff`].
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize, Constructor,
)]
pub struct ReconnectionBackoffPolicy {
    /// Initial backoff millisecond duration after the first `Stream` disconnection.
    ///
    /// This value then scales with the `backoff_multiplier` in the case of repeated failed
    /// `Stream` reconnection attempts.
    pub backoff_ms_initial: u64,

    /// Scaling factor for the backoff duration in the case of repeated `Stream` reconnection
    /// attempts.
    pub backoff_multiplier: u8,

    /// Maximum possible backoff duration between reconnection attempts.
    pub backoff_ms_max: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
struct ReconnectionState {
    policy: ReconnectionBackoffPolicy,
    backoff_ms_current: u64,
}

impl From<ReconnectionBackoffPolicy> for ReconnectionState {
    fn from(policy: ReconnectionBackoffPolicy) -> Self {
        Self {
            backoff_ms_current: policy.backoff_ms_initial,
            policy,
        }
    }
}

impl ReconnectionState {
    fn reset_backoff(&mut self) {
        self.backoff_ms_current = self.policy.backoff_ms_initial;
    }

    fn multiply_backoff(&mut self) {
        let next = self.backoff_ms_current * self.policy.backoff_multiplier as u64;
        let next_capped = std::cmp::min(next, self.policy.backoff_ms_max);
        self.backoff_ms_current = next_capped;
    }

    fn generate_sleep_future(&self) -> tokio::time::Sleep {
        let sleep_duration = std::time::Duration::from_millis(self.backoff_ms_current);
        tokio::time::sleep(sleep_duration)
    }
}
