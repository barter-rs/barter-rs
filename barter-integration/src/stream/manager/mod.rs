use crate::stream::manager::routing::route_stream;
use component_map::{ComponentMap, WithArgs};
use futures::Stream;
use std::{fmt::Debug, hash::Hash};
use tokio::sync::oneshot;
// Re-exports
pub use routing::{RoutingStreamHandle, RoutingStreamManager};

mod routing;
mod task;

/// Manages a collection of streams keyed by a unique identifier.
///
/// [`StreamManager`] provides lifecycle management for multiple streams, allowing them to be
/// initialised with associated arguments and later converted into routed streams that run
/// in background tasks.
///
/// # Type Parameters
/// * `StKey` - Unique identifier for each stream (e.g., exchange name, subscription ID)
/// * `StArgs` - Configuration arguments required to initialise each stream
/// * `St` - The stream type being managed
/// * `FnStInit` - Async function for initialising streams given a key and arguments
///
/// # Example
/// ```rust,ignore
/// use barter_integration::stream::manager::StreamManager;
/// use futures::stream;
///
/// #[derive(Clone, Debug, Eq, PartialEq, Hash)]
/// enum StreamKey {
///     StreamA,
///     StreamB,
/// }
///
/// #[derive(Clone)]
/// struct StreamArgs {
///     url: String,
/// }
///
/// async fn init_stream(
///     _key: &StreamKey,
///     _args: &StreamArgs,
/// ) -> Result<impl futures::Stream<Item = String>, std::io::Error> {
///     Ok(stream::iter(vec!["data1".to_string(), "data2".to_string()]))
/// }
///
/// let keyed_stream_args = vec![
///     (StreamKey::StreamA, StreamArgs { url: "ws://example-a.com".into() }),
///     (StreamKey::StreamB, StreamArgs { url: "ws://example-b.com".into() }),
/// ];
///
/// let manager = StreamManager::init(keyed_stream_args, init_stream).await.unwrap();
/// ```
#[derive(Debug)]
pub struct StreamManager<StKey, StArgs, St, FnStInit> {
    pub map: ComponentMap<StKey, StArgs, St, FnStInit>,
}





impl<StKey, StArgs, St, FnStInit> StreamManager<StKey, StArgs, St, FnStInit> {
    /// Initialises a [`StreamManager`] with the provided keyed stream arguments.
    ///
    /// Initialises all streams using the provided `stream_init` function. Each stream
    /// is associated with a unique key and its initialisation arguments.
    ///
    /// # Arguments
    /// * `args` - Iterator of `(StKey, StArgs)` tuples defining the streams to initialise
    /// * `stream_init` - Async function that initialises a stream given a key and arguments
    ///
    /// # Errors
    /// Returns an error if any stream fails to initialise. The error type is determined by
    /// the `stream_init` function.
    pub async fn init<Err>(
        args: impl IntoIterator<Item = (StKey, StArgs)>,
        stream_init: FnStInit,
    ) -> Result<Self, Err>
    where
        StKey: Clone + Eq + Hash,
        StArgs: Clone,
        FnStInit: AsyncFn(&StKey, &StArgs) -> Result<St, Err> + Clone,
    {
        let inner = ComponentMap::try_init_async(args, stream_init).await?;
        Ok(Self { map: inner })
    }

    /// Transforms the `StreamManager` into a `RoutingStreamManager`.
    ///
    /// Spawns tokio tasks for each stream and routes them using the provided routing function.
    /// The routing continues until either the stream ends, an error occurs, or the task is shut down.
    ///
    /// # Arguments
    /// * `runtime` - Tokio runtime handle on which to spawn the background tasks
    /// * `route_factory` - Factory function that creates a routing handler for each stream key
    pub fn route_streams<RouteFactory, FnRoute, StInitErr, RouteErr>(
        self,
        runtime: tokio::runtime::Handle,
        route_factory: RouteFactory,
    ) -> RoutingStreamManager<
        StKey,
        StArgs,
        impl AsyncFn(&StKey, &StArgs) -> Result<RoutingStreamHandle, StInitErr> + Clone,
    >
    where
        StKey: Clone + Eq + Hash + Send + 'static,
        StArgs: Clone,
        FnStInit: AsyncFn(&StKey, &StArgs) -> Result<St, StInitErr> + Clone + 'static,
        St: Stream + Send + 'static,
        St::Item: Send,
        RouteFactory: Fn(&StKey) -> FnRoute + Clone + 'static,
        FnRoute: FnMut(&StKey, St::Item) -> Result<(), RouteErr> + Send + 'static,
        StInitErr: Send + 'static,
        RouteErr: Send + 'static,
    {
        let Self {
            map:
                ComponentMap {
                    map,
                    init: stream_init,
                },
        } = self;

        let map = map
            .into_iter()
            .map(
                |(
                    key,
                    WithArgs {
                        component: stream,
                        args,
                    },
                )| {
                    let handle = route_stream(&runtime, &key, stream, &route_factory);
                    (
                        key,
                        WithArgs {
                            args,
                            component: handle,
                        },
                    )
                },
            )
            .collect();

        let init_routed_stream_handle = move |key: &StKey, args: &StArgs| {
            let runtime = runtime.clone();
            let stream_init = stream_init.clone();
            let route_factory = route_factory.clone();
            let key = key.clone();
            let args = args.clone();

            async move {
                let stream = stream_init(&key, &args).await?;
                Ok(route_stream(&runtime, &key, stream, &route_factory))
            }
        };

        RoutingStreamManager {
            components: ComponentMap::new(map, init_routed_stream_handle),
        }
    }
}
