use component_map::{ComponentMap, Keyed, WithArgs};
use std::{collections::HashMap, future::Future, hash::Hash};

/// Todo: rust docs explaining what this is. One sentence is fine. Max three if required. Might want to
///   At the end mention it uses ComponentMap under the hood to manage the Key + Arg -> Update mechanics
#[derive(Debug)]
pub struct TokioTaskManager<Key, Args, FnInitFut, FnInitTask> {
    tasks: ComponentMap<Key, TaskArgs<Args, FnInitFut>, tokio::task::JoinHandle<()>, FnInitTask>,
}

#[derive(Clone, Debug)]
struct TaskArgs<Args, FnInitFut> {
    args: Args,
    init_future: FnInitFut,
}

impl<Key, Args, FnInitFut, FnInitTask> TokioTaskManager<Key, Args, FnInitFut, FnInitTask> {
    /// Todo: rust docs explaining what this is. One sentence is fine. Max two if required.
    pub fn tasks(
        &self,
    ) -> &HashMap<Key, WithArgs<TaskArgs<Args, FnInitFut>, tokio::task::JoinHandle<()>>> {
        &self.tasks.map
    }

    /// Reinitialise all Tokio tasks using their associated Args.
    ///
    /// Returns an iterator of results for each task reinitialisation. // Todo: does this return the previous "components", if so make it more obvious in the docs
    pub async fn reinit_all<Err>(
        &mut self,
    ) -> impl Iterator<Item = Keyed<&Key, Result<tokio::task::JoinHandle<()>, Err>>>
    where
        FnInitTask: AsyncFn(&Key, &TaskArgs<Args, FnInitFut>) -> Result<tokio::task::JoinHandle<()>, Err>
            + Clone,
    {
        self.tasks.try_reinit_all_async().await
    }

    /// Reinitialise a Tokio task using it's associated Args.
    ///
    /// Returns an iterator of results for each requested key. If a key doesn't exist,
    /// the result will be `None`. // Todo: does this return the previous "components", if so make it more obvious in the docs
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
    /// // Todo: explain the return type in a simple one sentence.
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
        // Get a reference to the init_future from any existing entry
        // All entries should have the same init_future
        let init_future = self
            .tasks
            .map
            .values()
            .next()
            .map(|with_args| with_args.args.init_future.clone());

        // Wrap user args with TokioTaskArgs
        let wrapped_updates = updates.into_iter().map(move |(key, args)| {
            let init_future = init_future
                .as_ref()
                .expect("Cannot update: no existing tasks to get init_future from")
                .clone();
            (key, TaskArgs { args, init_future })
        });

        // Use ComponentMap's try_update_async and unwrap the user args
        self.tasks
            .try_update_async(wrapped_updates)
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

    Ok(TokioTaskManager { tasks })
}
