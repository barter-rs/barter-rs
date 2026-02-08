// use std::hash::Hash;
// use component_map::{ComponentMap, WithArgs};
// use tokio::sync::oneshot;
//
// #[derive(Debug)]
// pub struct TokioTaskManager<Key, Args, FnInit> {
//     pub runtime: tokio::runtime::Handle,
//     pub tasks: ComponentMap<Key, Args, TokioTaskHandle, FnInit>
// }
//
// impl<Key, Args, FnInit> TokioTaskManager<Key, Args, FnInit> {
//     pub fn new(tasks: impl Into<ComponentMap<Key, Args, TokioTaskHandle, FnInit>>) -> Self {
//         Self {
//             runtime: tokio::runtime::Handle::current(),
//             tasks: tasks.into()
//         }
//
//     }
//
//     pub fn new_with_runtime(
//         runtime: tokio::runtime::Handle,
//         tasks: impl Into<ComponentMap<Key, Args, TokioTaskHandle, FnInit>>
//     ) -> Self {
//         Self {
//             runtime,
//             tasks: tasks.into()
//         }
//
//     }
//
//     pub async fn init_new<T, Err>(
//         entries: impl IntoIterator<Item = (Key, Args)>,
//         init: FnInit,
//     ) -> Result<Self, Err>
//     where
//         Key: Clone + Eq + Hash,
//         Args: Clone,
//         FnInit: AsyncFn(&Key, &Args) -> Result<T, Err> + Clone,
//     {
//         ComponentMap::try_init_async(entries, Self::spawn)
//             .await
//             .map(Self::new)
//     }
//
//     pub async fn init_new_new<FnInitFactory, T, Err>(
//         entries: impl IntoIterator<Item = (Key, Args)>,
//         init_factory: FnInitFactory,
//     ) -> Result<Self, Err>
//     where
//         Key: Clone + Eq + Hash,
//         Args: Clone,
//         FnInit: AsyncFn(&Key, &Args) -> Result<T, Err> + Clone,
//         FnInitFactory: Fn(&Key) -> FnInit,
//     {
//
//
//         ComponentMap::try_init_async(entries, Self::spawn)
//             .await
//             .map(Self::new)
//     }
//
//     async fn spawn<FnInitFactory, T, Err>(
//         key: &Key,
//         args: &Args,
//         init_factory: FnInitFactory,
//         handle: &tokio::runtime::Handle,
//     ) -> Result<TokioTaskHandle, Err>
//     where
//         FnInit: AsyncFn(&Key, &Args) -> Result<T, Err> + Clone,
//         FnInitFactory: Fn(&Key) -> FnInit,
//     {
//         // Todo:
//         //   - Not sure shutdown works at this level of abstraction... should be inside Args!!!1
//         //   - Could create "Engine" that listens to config (arg changes) and re-spawns tasks.
//         //      '--> Engine be impl Processor, audit stream can be Result<ConfigUpdate, ConfigError> etc.
//         //    '--> essentially a framework for managing tasks and re-creating them when there are config update
//         //     '--> struct ComponentManager<Index, Arg, GlobalContext> (or similar)
//         //      '--> this is basically Spring / Actor framework territory
//         //       '--> this task manager would be the component engine
//
//         let init = init_factory(&key);
//         let future = init(key, args).await?;
//
//
//         Ok(TokioTaskHandle {
//             handle: handle.spawn(future),
//             // shutdown_tx: (),
//         })
//     }
//
//
//
//     /// Initialise a `TokioTaskManager` with the provided task entries and init function.
//     pub async fn init<Err>(
//         entries: impl IntoIterator<Item = (Key, Args)>,
//         init: FnInit,
//     ) -> Result<Self, Err>
//     where
//         Key: Clone + Eq + Hash,
//         Args: Clone,
//         FnInit: AsyncFn(&Key, &Args) -> Result<TokioTaskHandle, Err> + Clone,
//     {
//         ComponentMap::try_init_async(entries, init)
//             .await
//             .map(Self::new)
//     }
//
//     /// Gracefully shuts down the task associated with the Key.
//     ///
//     /// Sends a shutdown signal to the task and returns a join handle
//     /// that can be awaited to ensure the task completes gracefully.
//     pub fn shutdown(&mut self, key: &Key) -> Option<WithArgs<Args, tokio::task::JoinHandle<()>>>
//     where
//         Key: Eq + Hash,
//     {
//         self.tasks
//             .map
//             .remove(key)
//             .map(|WithArgs { component, args }| WithArgs {
//                 component: component.shutdown(),
//                 args,
//             })
//     }
//
//     /// Gracefully shuts down all tasks.
//     ///
//     /// Sends a shutdown signal to all tasks and returns an iterator of join handles.
//     /// Each handle can be awaited to ensure the task completes gracefully.
//     pub fn shutdown_all(
//         self,
//     ) -> impl Iterator<Item = (Key, WithArgs<Args, tokio::task::JoinHandle<()>>)> {
//         self.tasks
//             .map
//             .into_iter()
//             .map(|(key, WithArgs { component, args })| {
//                 (
//                     key,
//                     WithArgs {
//                         component: component.shutdown(),
//                         args,
//                     },
//                 )
//             })
//     }
// }
//
// pub struct TokioTaskArgs {
//
// }
//
// /// Handle for a tokio task.
// ///
// /// Provides methods to gracefully shutdown or forcefully abort the task.
// ///
// /// # Shutdown vs Abort
// /// - [`shutdown`](Self::shutdown) - Gracefully signals the task to stop and returns a join handle.
// /// - [`abort`](Self::abort) - Sends shutdown signal but immediately aborts the task without waiting.
// pub struct TokioTaskHandle {
//     pub handle: tokio::task::JoinHandle<()>,
//     // pub shutdown_tx: oneshot::Sender<()>,
// }
//
// impl TokioTaskHandle {
//     /// Forcefully aborts the task.
//     ///
//     /// Internally a graceful shutdown signal is sent, but immediately after the background task
//     /// is aborted so the task may not have time to clean up resources.
//     pub fn abort(self) {
//         let handle = self.shutdown();
//         handle.abort()
//     }
//
//     /// Gracefully shuts down the task.
//     ///
//     /// Sends a shutdown signal to the background task and returns a join handle that can be
//     /// awaited to ensure the task completes.
//     pub fn shutdown(self) -> tokio::task::JoinHandle<()> {
//         let _ = self.shutdown_tx.send(());
//         self.handle
//     }
// }
