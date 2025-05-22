use std::collections::HashMap;
use jackbot_execution::order::id::StrategyId;

#[derive(Debug, Default)]
pub struct StrategyRegistry<S> {
    strategies: HashMap<StrategyId, S>,
}

impl<S> StrategyRegistry<S> {
    pub fn new() -> Self {
        Self { strategies: HashMap::new() }
    }

    pub fn register(&mut self, id: StrategyId, strategy: S) {
        self.strategies.insert(id, strategy);
    }

    pub fn get(&self, id: &StrategyId) -> Option<&S> {
        self.strategies.get(id)
    }

    pub fn get_mut(&mut self, id: &StrategyId) -> Option<&mut S> {
        self.strategies.get_mut(id)
    }

    pub fn remove(&mut self, id: &StrategyId) -> Option<S> {
        self.strategies.remove(id)
    }
}

impl<S> IntoIterator for StrategyRegistry<S> {
    type Item = (StrategyId, S);
    type IntoIter = std::collections::hash_map::IntoIter<StrategyId, S>;
    fn into_iter(self) -> Self::IntoIter {
        self.strategies.into_iter()
    }
}
