use component_map::{ComponentMap, Keyed, WithArgs};
use std::{future::Future, hash::Hash};

trait Processor {}

// impl Processor<Update>

// Todo: Create a Future which can manage the updating of all threads
//        similar be run_sync and run_async, etc
//         perhaps a run_sync cna be with std::threads

struct Update<Key, Args> {
    key: Key, // TaskFilter
    kind:  ComponentUpdateKind<Args>
}

enum ComponentUpdateKind<Args> {
    Upsert(Args),
    Remove,
    Shutdown,
}

enum TaskFilter {

}

// Todo: aside: generalise "filter" for any State (ie/ InstrumentFilter, etc)
trait Filterable<Filter> {
    type State;
    fn filtered(&self, filter: &Filter) -> impl Iterator<Item = &Self::State>;
    fn filtered_mut(&mut self, filter: &Filter) -> impl Iterator<Item = &mut Self::State>;
}

struct HigherAbstractionThanTokioTaskManager {

}

impl Filterable for HigherAbstractionThanTokioTaskManager {

}



/// Manages a collection of Tokio tasks keyed by a unique identifier.
///
/// Provides lifecycle management for multiple tasks, allowing them to be initialised, reinitialised,
/// and updated with new arguments. Uses [`ComponentMap`] under the hood to manage the mapping between
/// keys, arguments, and task handles.
#[derive(Debug)]
pub struct TokioTaskManager<Key, Args, FnInitFut, FnInitTask> {
    tasks: ComponentMap<Key, TaskArgs<Args, FnInitFut>, tokio::task::JoinHandle<()>, FnInitTask>,
    init_future: FnInitFut,
}

#[derive(Clone, Debug)]
struct TaskArgs<Args, FnInitFut> {
    args: Args,
    init_future: FnInitFut,
}

impl<Key, Args, FnInitFut, FnInitTask> TokioTaskManager<Key, Args, FnInitFut, FnInitTask> {
    /// Reinitialise all Tokio tasks using their associated Args.
    ///
    /// Replaces each existing task handle with a newly spawned task and returns the previous task handles.
    pub async fn reinit_all<Err>(
        &mut self,
    ) -> impl Iterator<Item = Keyed<&Key, Result<tokio::task::JoinHandle<()>, Err>>>
    where
        FnInitTask: AsyncFn(&Key, &TaskArgs<Args, FnInitFut>) -> Result<tokio::task::JoinHandle<()>, Err>
            + Clone,
    {
        self.tasks.try_reinit_all_async().await
    }

    /// Reinitialise specific Tokio tasks using their associated Args.
    ///
    /// Replaces existing task handles with newly spawned tasks, and returns the previous task handles.
    /// If a requested key doesn't exist, the result will be `None`.
    pub async fn reinit<Err>(
        &mut self,
        keys: impl IntoIterator<Item = Key>,
    ) -> impl Iterator<Item = Keyed<Key, Option<Result<tokio::task::JoinHandle<()>, Err>>>>
    where
        Key: Eq + Hash + Clone,
        FnInitTask: AsyncFn(&Key, &TaskArgs<Args, FnInitFut>) -> Result<tokio::task::JoinHandle<()>, Err>
            + Clone,
    {
        self.tasks.try_reinit_async(keys).await
    }

    /// Update Tokio task Args and then reinitialise the tasks.
    ///
    /// Replaces existing task handles with the newly spawned tasks, and returns the previous task
    /// handles.
    ///
    /// If an update key didn't previously exist, the returned previous task will be `None`.
    pub async fn update<Err>(
        &mut self,
        updates: impl IntoIterator<Item = (Key, Args)>,
    ) -> impl Iterator<Item = Keyed<Key, Option<Result<WithArgs<Args, tokio::task::JoinHandle<()>>, Err>>>>
    where
        Key: Clone + Eq + Hash,
        FnInitFut: Clone,
        FnInitTask: AsyncFn(&Key, &TaskArgs<Args, FnInitFut>) -> Result<tokio::task::JoinHandle<()>, Err>
            + Clone,
    {
        let init_future = self.init_future.clone();

        let updates = updates.into_iter().map(move |(key, args)| {
            (
                key,
                TaskArgs {
                    args,
                    init_future: init_future.clone(),
                },
            )
        });

        self.tasks
            .try_update_async(updates)
            .await
            .map(|Keyed { key, value }| {
                Keyed::new(
                    key,
                    value.map(|result| {
                        result.map(|with_args| WithArgs {
                            component: with_args.component,
                            args: with_args.args.args,
                        })
                    }),
                )
            })
    }
}

pub async fn init_tokio_task_manager<Key, Args, FnInitFut, Fut, Err>(
    runtime: tokio::runtime::Handle,
    entries: impl IntoIterator<Item = (Key, Args)>,
    init_future: FnInitFut,
) -> Result<
    TokioTaskManager<
        Key,
        Args,
        FnInitFut,
        impl AsyncFn(&Key, &TaskArgs<Args, FnInitFut>) -> Result<tokio::task::JoinHandle<()>, Err>,
    >,
    Err,
>
where
    Key: Clone + Eq + Hash,
    Args: Clone,
    FnInitFut: AsyncFn(&Key, &Args) -> Result<Fut, Err> + Clone,
    Fut: Future<Output = ()> + Send + 'static,
{
    let entries = entries.into_iter().map(|(key, args)| {
        (
            key,
            TaskArgs {
                args,
                init_future: init_future.clone(),
            },
        )
    });

    let init_task = move |key: &Key, task_args: &TaskArgs<Args, FnInitFut>| {
        let runtime = runtime.clone();
        let init_future = task_args.init_future.clone();
        let key = key.clone();
        let args = task_args.args.clone();

        async move {
            let future = (init_future)(&key, &args).await?;
            Ok(runtime.spawn(future))
        }
    };

    let tasks = ComponentMap::try_init_async(entries, init_task).await?;

    Ok(TokioTaskManager { tasks, init_future })
}
