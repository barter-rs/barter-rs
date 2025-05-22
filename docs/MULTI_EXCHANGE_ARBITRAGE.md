# Multi-Exchange Aggregation and Arbitrage Framework

This module provides utilities for aggregating order books across multiple exchanges and detecting simple arbitrage opportunities. Books can be combined with custom weights to reflect liquidity, fees or latency considerations.

## Order Book Aggregation

`OrderBookAggregator` collects a list of `ExchangeBook` items. Each `ExchangeBook` contains an `ExchangeId`, a shared reference to an order book, and an optional weight. Calling `aggregate(depth)` merges the books into a single snapshot taking the specified number of levels.

```rust
use jackbot_data::books::aggregator::{OrderBookAggregator, ExchangeBook};
use rust_decimal_macros::dec;

let agg = OrderBookAggregator::new([
    ExchangeBook { exchange: ExchangeId::BinanceSpot, book: book_a, weight: dec!(2) },
    ExchangeBook { exchange: ExchangeId::Coinbase, book: book_b, weight: dec!(1) },
]);

let snapshot = agg.aggregate(5);
```

## Position Tracking

`jackbot-risk` exposes a `PositionTracker` for monitoring positions across exchanges. It records net quantity per `(ExchangeId, Instrument)` pair and can enforce limits via the risk alert system.

```rust
use jackbot_risk::{position_tracker::PositionTracker, alert::VecAlertHook};
use rust_decimal_macros::dec;

let mut tracker = PositionTracker::new();
tracker.update(ExchangeId::BinanceSpot, InstrumentIndex(0), dec!(5));
```

## Configurable Strategies

Strategies can load parameters via `jackbot-strategy::StrategyConfig`. Combine this with the aggregation and position tracking utilities to build arbitrage strategies tailored to your requirements.
