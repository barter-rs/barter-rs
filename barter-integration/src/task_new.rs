use component_map::{ComponentMap, Keyed, WithArgs};
use std::{collections::HashMap, future::Future, hash::Hash};

#[derive(Debug)]
pub struct TokioTaskManager<Key, Args, FnInitFut, FnInitTask> {
    tasks:
        ComponentMap<Key, TokioTaskArgs<Args, FnInitFut>, tokio::task::JoinHandle<()>, FnInitTask>,
}

#[derive(Clone, Debug)]
pub struct TokioTaskArgs<Args, FnInitFut> {
    pub args: Args,
    pub init_future: FnInitFut,
}

impl<Key, Args, FnInitFut, FnInitTask> TokioTaskManager<Key, Args, FnInitFut, FnInitTask> {
    pub fn tasks(&self) -> &HashMap<Key, WithArgs<Args, tokio::task::JoinHandle<()>>> {
        &self.tasks.map
    }

    pub async fn reinit_all<Err>(
        &mut self,
    ) -> impl Iterator<Item = Keyed<&Key, Result<Comp, Error>>> // mimic try_reinit_all_async return type
    {
    }

    pub async fn reinit<Err>(
        &mut self,
        keys: impl IntoIterator<Item = Key>,
    ) -> impl Iterator<Item = Keyed<Key, Option<Result<Comp, Error>>>> // mimic try_reinit_async return type
    {
    }

    pub async fn update<Err>(
        &mut self,
        updates: impl IntoIterator<Item = (Key, Args)>,
    ) -> impl Iterator<Item = Keyed<Key, Option<Result<WithArgs<Args, Comp>, Error>>>> // mimic try_update_async return type
    {
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
        impl AsyncFn(&Key, &TokioTaskArgs<Args, FnInitFut>) -> Result<tokio::task::JoinHandle<()>, Err>,
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
            TokioTaskArgs {
                args,
                init_future: init_future.clone(),
            },
        )
    });

    let init_task = move |key: &Key, task_args: &TokioTaskArgs<Args, FnInitFut>| {
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
