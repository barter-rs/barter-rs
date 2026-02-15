# ComponentMap Redesign Options - Usage Comparison

This document shows how each option would be used to implement `TokioTaskManager`.

## Option 2: Context Parameter

### Key Features
- Adds a `Context` generic parameter to `ComponentMap`
- Init function receives `(&Key, &Args, &Context)` instead of just `(&Key, &Args)`
- Context is shared across all operations without being stored per-component

### Usage Example

```rust
pub struct TokioTaskManager<Key, Args, FnInitFut, FnInit> {
    tasks: ComponentMap<
        Key,
        Args,  // Clean user Args, no wrapping needed!
        tokio::task::JoinHandle<()>,
        (tokio::runtime::Handle, FnInitFut),  // Context tuple
        FnInit
    >
}

pub async fn init_tokio_task_manager<Key, Args, FnInitFut, Fut>(
    entries: impl IntoIterator<Item = (Key, Args)>,
    runtime: tokio::runtime::Handle,
    init_future: FnInitFut,
) -> TokioTaskManager<Key, Args, FnInitFut, FnInit>
where
    Key: Clone + Eq + Hash,
    Args: Clone,
    FnInitFut: AsyncFn(&Key, &Args) -> Fut + Clone,
    Fut: Future<Output = ()> + Send + 'static,
{
    let context = (runtime.clone(), init_future);

    let tasks = ComponentMap::init_async(
        entries,
        context,
        |key, args, (runtime, init_future)| async move {
            let future = init_future(key, args).await;
            runtime.spawn(future)
        }
    ).await;

    TokioTaskManager { tasks }
}

impl<Key, Args, FnInitFut, FnInit> TokioTaskManager<Key, Args, FnInitFut, FnInit> {
    // Methods are clean - no TaskArgs wrapping needed!
    pub async fn reinit_all(&mut self) {
        self.tasks.reinit_all_async().await
    }

    pub async fn update(&mut self, updates: impl IntoIterator<Item = (Key, Args)>) {
        // Just pass through - no wrapping needed!
        self.tasks.update_async(updates).await
    }
}
```

### Pros
- ✅ Clean user-facing `Args` type (no wrapping)
- ✅ Explicit context in type signature
- ✅ Simple function signatures (no `impl AsyncFn` in return types)
- ✅ Context is mutable if needed

### Cons
- ❌ Additional generic parameter on `ComponentMap`
- ❌ Context must be cloneable for each operation

---

## Option 4: Separated Init and Storage

### Key Features
- Separates initial component creation from storage
- `init_components()` is a standalone function that returns a `HashMap`
- You provide different init functions for construction vs reinitialisation

### Usage Example

```rust
pub struct TokioTaskManager<Key, Args, FnInit> {
    tasks: ComponentMap<
        Key,
        TaskArgs<Args, FnInitFut>,  // Still need wrapping
        tokio::task::JoinHandle<()>,
        FnInit
    >
}

pub async fn init_tokio_task_manager<Key, Args, FnInitFut, Fut>(
    entries: impl IntoIterator<Item = (Key, Args)>,
    runtime: tokio::runtime::Handle,
    init_future: FnInitFut,
) -> TokioTaskManager<Key, Args, FnInit>
where
    Key: Clone + Eq + Hash,
    Args: Clone,
    FnInitFut: AsyncFn(&Key, &Args) -> Fut + Clone,
    Fut: Future<Output = ()> + Send + 'static,
{
    // Phase 1: Init components with wrapped args using one-time init function
    let wrapped_entries = entries.into_iter().map(|(key, args)| {
        (key, TaskArgs { args, init_future: init_future.clone() })
    });

    let map = ComponentMap::init_components(
        wrapped_entries,
        |key, task_args| async move {
            let future = task_args.init_future(key, &task_args.args).await;
            runtime.spawn(future)
        }
    ).await;

    // Phase 2: Create ComponentMap with the reinit function
    let reinit_fn = move |key, task_args| async move {
        let future = task_args.init_future(key, &task_args.args).await;
        runtime.spawn(future)
    };

    let tasks = ComponentMap::new(map, reinit_fn);

    TokioTaskManager { tasks }
}

impl<Key, Args, FnInit> TokioTaskManager<Key, Args, FnInit> {
    // Still need wrapping in update method
    pub async fn update(&mut self, updates: impl IntoIterator<Item = (Key, Args)>) {
        let init_future = // ... get from somewhere
        let wrapped_updates = updates.into_iter().map(|(key, args)| {
            (key, TaskArgs { args, init_future: init_future.clone() })
        });
        self.tasks.update_async(wrapped_updates).await
    }
}
```

### Pros
- ✅ Flexible - different logic for init vs reinit
- ✅ No additional generic parameters
- ✅ Clear separation of concerns

### Cons
- ❌ Still requires `TaskArgs` wrapping
- ❌ More verbose usage (two-phase construction)
- ❌ Doesn't solve the core problem of needing context during updates

---

## Recommendation

**Option 2 (Context Parameter)** is superior because:

1. **Eliminates wrapping** - User `Args` stay clean throughout
2. **Simpler type signatures** - No `impl AsyncFn` in return types
3. **More ergonomic** - Single-phase construction
4. **Solves the core problem** - Context is available for all operations (init, reinit, update)

Option 4 doesn't fully solve the problem since you still need to wrap args to carry the context.