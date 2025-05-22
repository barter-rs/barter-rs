use jackbot_strategy::{RecordingStrategy, CountingStrategy, Strategy, StrategyConfig};

#[test]
fn recording_strategy_collects_events() {
    let mut strat = RecordingStrategy::default();
    strat.on_start(&StrategyConfig { parameters: Default::default() });
    strat.on_event(&1);
    strat.on_event(&2);
    strat.on_stop();
    assert_eq!(strat.events, vec![1, 2]);
}

#[test]
fn counting_strategy_counts_events() {
    let mut strat = CountingStrategy::default();
    for _ in 0..5 {
        strat.on_event(&"event");
    }
    assert_eq!(strat.count, 5);
}
