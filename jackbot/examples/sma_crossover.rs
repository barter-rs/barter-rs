use Jackbot::strategy::{framework::Strategy, registry::StrategyRegistry, DefaultStrategy};
use Jackbot::technical::{sma, ema, rsi};
use jackbot_execution::order::id::StrategyId;

fn main() {
    let data = vec![1.0,2.0,3.0,4.0,5.0,6.0,7.0];
    let short = sma(3, &data);
    let long = sma(5, &data);
    println!("short SMA: {:?}", short);
    println!("long SMA: {:?}", long);

    let mut registry = StrategyRegistry::new();
    let strat = DefaultStrategy::<()>::default();
    registry.register(strat.id.clone(), strat);
    println!("registered strategies: {}", registry.into_iter().count());
}
