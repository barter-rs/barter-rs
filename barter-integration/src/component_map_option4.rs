use futures::future::join_all;
use std::collections::HashMap;
use std::hash::Hash;

/// A component map with separated initialisation and storage phases.
///
/// This design separates the initial construction of components from the storage and
/// reinitialisation logic. During init, you can use a different function than the one
/// used for reinitialisation/updates, allowing for flexible argument wrapping.
#[derive(Debug, Clone)]
pub struct ComponentMap<Key, Args, Comp, FnInit> {
    pub map: HashMap<Key, WithArgs<Args, Comp>>,
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
impl<Key, Args, Comp, FnInit> ComponentMap<Key, Args, Comp, FnInit> {
    /// Create a ComponentMap from an already-initialised map and an init function.
    ///
    /// This constructor allows you to separate the initial component creation from
    /// the reinitialisation logic. Useful when init and reinit need different contexts.
    pub fn new(map: HashMap<Key, WithArgs<Args, Comp>>, init: FnInit) -> Self {
        Self { map, init }
    }

    /// Initialise components from entries without creating a ComponentMap yet.
    ///
    /// This is the "Phase 1" function that creates components. It's separate from
    /// the ComponentMap storage, allowing you to use different init logic during
    /// construction vs reinitialisation.
    ///
    /// Returns a HashMap ready to be passed to `ComponentMap::new()`.
    pub async fn init_components<InitFn>(
        entries: impl IntoIterator<Item = (Key, Args)>,
        init_fn: InitFn,
    ) -> HashMap<Key, WithArgs<Args, Comp>>
    where
        Key: Eq + Hash,
        InitFn: AsyncFn(&Key, &Args) -> Comp + Clone,
    {
        let components_fut = entries.into_iter().map(|(key, args)| {
            let init_fn = init_fn.clone();
            async move {
                let component = init_fn(&key, &args).await;
                (key, WithArgs { component, args })
            }
        });

        join_all(components_fut).await.into_iter().collect()
    }

    /// Convenience method that combines `init_components` and `new`.
    ///
    /// Use this when your init and reinit functions are the same.
    pub async fn init_async(entries: impl IntoIterator<Item = (Key, Args)>, init: FnInit) -> Self
    where
        Key: Eq + Hash,
        FnInit: AsyncFn(&Key, &Args) -> Comp + Clone,
    {
        let map = Self::init_components(entries, init.clone()).await;
        Self::new(map, init)
    }

    /// Reinitialise all components using their stored args.
    ///
    /// Returns an iterator of the previous components.
    pub async fn reinit_all_async(&mut self) -> impl Iterator<Item = Keyed<&Key, Comp>>
    where
        FnInit: AsyncFn(&Key, &Args) -> Comp + Clone,
    {
        let next_components_fut = self.map.iter().map(|(key, with_args)| {
            let init = self.init.clone();
            async move { init(key, &with_args.args).await }
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
        FnInit: AsyncFn(&Key, &Args) -> Comp + Clone,
    {
        let next_components_fut = keys.into_iter().map(|key| {
            let init = self.init.clone();
            let args = self.map.get(&key).map(|with_args| &with_args.args);

            async move {
                let component = match args {
                    Some(args) => Some(init(&key, args).await),
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
        FnInit: AsyncFn(&Key, &Args) -> Comp + Clone,
    {
        let updated_components_fut = updates.into_iter().map(|(key, args)| {
            let init = self.init.clone();
            async move {
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