# Market Making Engine

Jackbot's market making engine provides a set of utilities for building two-sided quoting strategies. It aims to keep order placement responsive while controlling inventory risk.

## Core Components

- **InventorySkewQuoter** – Generates bid and ask prices around a mid price, skewing the spread according to current inventory exposure.
- **optimize_spread** – Adjusts the base spread for volatility while enforcing a maximum.
- **RiskControls** – Simple inventory based gating for enabling or disabling trading.
- **PerformanceTracker** – Records realised PnL and executed trade count.
- **FlowToxicityDetector** – Analyses the ratio of buy and sell volume to detect toxic flow.
- **QuoteRefresher** – Tracks when quotes should be refreshed.
- **reactive_adjust** – Moves quotes in the direction of recent flow.
- **predictive_adjust** – Re-centres quotes around a forecasted mid price while keeping the spread constant.

## Example Configuration

```json
{
  "market_maker": {
    "target_spread": "2",
    "inventory_skew_factor": "0.5",
    "max_inventory_ratio": "0.25",
    "refresh_interval_secs": 10
  }
}
```

## Example Usage

```rust,no_run
use rust_decimal_macros::dec;
use jackbot::market_maker::{InventorySkewQuoter, RiskControls, PerformanceTracker};
use jackbot_execution::market_making::{FlowToxicityDetector, QuoteRefresher, reactive_adjust};
use chrono::{Duration, Utc};

let quoter = InventorySkewQuoter::new(dec!(2), dec!(0.5));
let quote = quoter.quote(dec!(100), dec!(0.2));

let risk = RiskControls::new(dec!(0.25));
assert!(risk.check_inventory(dec!(0.2)));

let detector = FlowToxicityDetector::new(dec!(0.6));
let trades = vec![(jackbot_execution::market_making::TradeSide::Buy, dec!(7)),
                 (jackbot_execution::market_making::TradeSide::Buy, dec!(3))];
let toxic = detector.is_toxic(&trades);

let mut refresher = QuoteRefresher::new(Duration::seconds(10));
let now = Utc::now();
if refresher.needs_refresh(now) {
    refresher.record_refresh(now);
}

let adjusted = reactive_adjust(quote, jackbot_execution::market_making::TradeSide::Buy, dec!(1));
println!("Adjusted quotes: {:?}", adjusted);
```

This example demonstrates how the various components fit together. The configuration snippet illustrates the key parameters that can be tuned for different strategies.
