use Jackbot::strategy::{registry::StrategyRegistry, DefaultStrategy};
use jackbot_execution::order::id::StrategyId;

#[test]
fn test_register_and_get() {
    let mut reg = StrategyRegistry::new();
    let strat: DefaultStrategy<()> = DefaultStrategy::default();
    let id = strat.id.clone();
    reg.register(id.clone(), strat);
    assert!(reg.get(&id).is_some());
    assert!(reg.remove(&id).is_some());
    assert!(reg.get(&id).is_none());
}
