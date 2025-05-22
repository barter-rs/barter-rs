use Jackbot::strategy::{framework::Strategy, registry::StrategyRegistry, DefaultStrategy};
use jackbot_ta::indicators::SimpleMovingAverage;
use jackbot_ta::signals::CrossOverSignal;
use jackbot_execution::order::id::StrategyId;

fn main() {
    let data = [1.0,2.0,3.0,4.0,5.0,6.0,7.0];
    let mut short = SimpleMovingAverage::new(3);
    let mut long = SimpleMovingAverage::new(5);
    for value in data {
        short.update(value.into());
        long.update(value.into());
    }
    println!("short SMA: {}", short.average());
    println!("long SMA: {}", long.average());

    let mut signal = CrossOverSignal::new();
    let sig = signal.update(short.average(), long.average());
    println!("signal: {:?}", sig);

    let mut registry = StrategyRegistry::new();
    let strat = DefaultStrategy::<()>::default();
    registry.register(strat.id.clone(), strat);
    println!("registered strategies: {}", registry.into_iter().count());
}
