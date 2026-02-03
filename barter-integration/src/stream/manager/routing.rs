use component_map::{ComponentMap, WithArgs};
use futures::{Stream, StreamExt};
use std::hash::Hash;
use tokio::sync::oneshot;

/// Manages a collection of streams being routed in tasks.
///
/// A `RoutingStreamManager` is created by the
/// [`StreamManager::route_streams`](super::StreamManager::route_streams) method.
#[derive(Debug)]
pub struct RoutingStreamManager<StKey, StArgs, FnInit> {
    pub components: ComponentMap<StKey, StArgs, RoutingStreamHandle, FnInit>,
}

impl<StKey, StArgs, FnInit> RoutingStreamManager<StKey, StArgs, FnInit> {
    /// Gracefully shuts down a specific stream by its key.
    ///
    /// Sends a shutdown signal to the stream's background task and returns a join handle
    /// that can be awaited to ensure the task completes.
    ///
    /// If the `StKey` is not found in the map, `None` is returned.
    pub fn shutdown(&mut self, key: &StKey) -> Option<WithArgs<StArgs, tokio::task::JoinHandle<()>>>
    where
        StKey: Eq + Hash,
    {
        self.components
            .map
            .remove(key)
            .map(|WithArgs { component, args }| WithArgs {
                component: component.shutdown(),
                args,
            })
    }

    /// Gracefully shuts down all routing stream tasks.
    ///
    /// Sends a shutdown signal to all routing stream tasks and returns an iterator of join handles
    /// for all running tasks. Each handle can be awaited to ensure graceful shutdown.
    pub fn shutdown_all(
        self,
    ) -> impl Iterator<Item = (StKey, WithArgs<StArgs, tokio::task::JoinHandle<()>>)> {
        self.components
            .map
            .into_iter()
            .map(|(key, WithArgs { component, args })| {
                (
                    key,
                    WithArgs {
                        component: component.shutdown(),
                        args,
                    },
                )
            })
    }
}

/// Handle for a stream routing task.
///
/// Provides methods to gracefully shut down or forcefully abort the stream's background task.
///
/// # Shutdown vs Abort
/// - [`shutdown`](Self::shutdown) - Gracefully signals the task to stop and returns a join handle
/// - [`abort`](Self::abort) - Immediately terminates the task without waiting
#[derive(Debug)]
pub struct RoutingStreamHandle {
    handle: tokio::task::JoinHandle<()>,
    shutdown_tx: oneshot::Sender<()>,
}

impl RoutingStreamHandle {
    /// Forcefully aborts the stream routing task.
    ///
    /// Internally a graceful shutdown signal is sent, but immediately after the background task
    /// is aborted so the task may not have time to clean up resources.
    pub fn abort(self) {
        let handle = self.shutdown();
        handle.abort()
    }

    /// Gracefully shuts down the stream routing task.
    ///
    /// Sends a shutdown signal to the background task and returns a join handle that can be
    /// awaited to ensure the task completes.
    pub fn shutdown(self) -> tokio::task::JoinHandle<()> {
        let _ = self.shutdown_tx.send(());
        self.handle
    }
}

pub(super) fn route_stream<StKey, St, RouteFactory, FnRoute, Err>(
    runtime: &tokio::runtime::Handle,
    key: &StKey,
    stream: St,
    route_factory: &RouteFactory,
) -> RoutingStreamHandle
where
    StKey: Clone + Send + 'static,
    St: Stream + Send + 'static,
    St::Item: Send,
    RouteFactory: Fn(&StKey) -> FnRoute,
    FnRoute: FnMut(&StKey, St::Item) -> Result<(), Err> + Send + 'static,
    Err: Send + 'static,
{
    let mut route = route_factory(key);
    let key = key.clone();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let handle = runtime.spawn(
        stream
            .take_until(shutdown_rx)
            .map(move |item| route(&key, item))
            .take_while(|result: &Result<(), Err>| std::future::ready(result.is_ok()))
            .for_each(|_| std::future::ready(())),
    );

    RoutingStreamHandle {
        handle,
        shutdown_tx,
    }
}
