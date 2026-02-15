use futures::future::join_all;
use std::collections::HashMap;
use std::hash::Hash;

/// A component map that includes shared context passed to init functions.
///
/// The `Context` parameter allows passing shared state (like runtime handles, factories, etc.)
/// to the init function without storing it per-component in the args.
#[derive(Debug, Clone)]
pub struct ComponentMap<Key, Args, Comp, Context, FnInit> {
    pub map: HashMap<Key, WithArgs<Args, Comp>>,
    pub context: Context,
    pub init: FnInit,
}

/// Wrapper that pairs a component with its associated args.
#[derive(Debug, Clone)]
pub struct WithArgs<Args, Comp> {
    pub args: Args,
    pub component: Comp,
}

/// Key-value pair used in iterators.
#[derive(Debug, Clone)]
pub struct Keyed<Key, Value> {
    pub key: Key,
    pub value: Value,
}

impl<Key, Value> Keyed<Key, Value> {
    pub fn new(key: Key, value: Value) -> Self {
        Self { key, value }
    }
}

// Async infallible implementation
impl<Key, Args, Comp, Context, FnInit> ComponentMap<Key, Args, Comp, Context, FnInit> {
    /// Initialise a new ComponentMap with the provided entries, context, and init function.
    ///
    /// The init function receives `(&Key, &Args, &Context)` allowing it to use shared
    /// context without storing it per-component.
    pub async fn init_async(
        args: impl IntoIterator<Item = (Key, Args)>,
        context: Context,
        init: FnInit,
    ) -> Self
    where
        Key: Eq + Hash,
        Context: Clone,
        FnInit: AsyncFn(&Key, &Args, &Context) -> Comp + Clone,
    {
        let components_fut = args.into_iter().map(|(key, args)| {
            let init = init.clone();
            let context = context.clone();
            async move {
                let component = init(&key, &args, &context).await;
                (key, WithArgs { component, args })
            }
        });

        let map = join_all(components_fut).await.into_iter().collect();

        Self { map, context, init }
    }

    /// Reinitialise all components using their stored args and the shared context.
    ///
    /// Returns an iterator of the previous components.
    pub async fn reinit_all_async(&mut self) -> impl Iterator<Item = Keyed<&Key, Comp>>
    where
        FnInit: AsyncFn(&Key, &Args, &Context) -> Comp + Clone,
        Context: Clone,
    {
        let next_components_fut = self.map.iter().map(|(key, with_args)| {
            let init = self.init.clone();
            let context = self.context.clone();
            async move { init(key, &with_args.args, &context).await }
        });

        let next_components = join_all(next_components_fut).await;

        self.map
            .iter_mut()
            .zip(next_components)
            .map(|((key, with_args), next)| {
                let prev = std::mem::replace(&mut with_args.component, next);
                Keyed::new(key, prev)
            })
    }

    /// Reinitialise specific components by their keys.
    ///
    /// Returns an iterator of the previous components. If a key doesn't exist, `None` is returned.
    pub async fn reinit_async(
        &mut self,
        keys: impl IntoIterator<Item = Key>,
    ) -> impl Iterator<Item = Keyed<Key, Option<Comp>>>
    where
        Key: Eq + Hash + Clone,
        FnInit: AsyncFn(&Key, &Args, &Context) -> Comp + Clone,
        Context: Clone,
    {
        let next_components_fut = keys.into_iter().map(|key| {
            let init = self.init.clone();
            let context = self.context.clone();
            let args = self.map.get(&key).map(|with_args| &with_args.args);

            async move {
                let component = match args {
                    Some(args) => Some(init(&key, args, &context).await),
                    None => None,
                };
                Keyed::new(key, component)
            }
        });

        let results = join_all(next_components_fut).await;

        results.into_iter().map(|keyed| {
            let prev = keyed.value.and_then(|next| {
                self.map
                    .get_mut(&keyed.key)
                    .map(|with_args| std::mem::replace(&mut with_args.component, next))
            });

            Keyed::new(keyed.key, prev)
        })
    }

    /// Update components with new args and reinitialise them.
    ///
    /// Returns an iterator where `None` indicates a newly created component, and `Some(prev)`
    /// contains the previous component that was replaced.
    pub async fn update_async(
        &mut self,
        updates: impl IntoIterator<Item = (Key, Args)>,
    ) -> impl Iterator<Item = Keyed<Key, Option<WithArgs<Args, Comp>>>>
    where
        Key: Clone + Eq + Hash,
        FnInit: AsyncFn(&Key, &Args, &Context) -> Comp + Clone,
        Context: Clone,
    {
        let updated_components_fut = updates.into_iter().map(|(key, args)| {
            let init = self.init.clone();
            let context = self.context.clone();
            async move {
                let component = init(&key, &args, &context).await;
                (key, WithArgs { component, args })
            }
        });

        join_all(updated_components_fut)
            .await
            .into_iter()
            .map(|(key, with_args)| {
                let prev = self.map.insert(key.clone(), with_args);
                Keyed::new(key, prev)
            })
    }
}
