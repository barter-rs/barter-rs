# Jackbot-Risk
Core risk management utilities for Jackbot trading systems.
It provides building blocks for exposure tracking, drawdown
management and correlation checks with simple alert hooks.

```rust
use jackbot_risk::{exposure::ExposureTracker, alert::VecAlertHook};
use jackbot_instrument::instrument::InstrumentIndex;
use rust_decimal_macros::dec;

let mut tracker: ExposureTracker<InstrumentIndex> = ExposureTracker::new();
tracker.update(InstrumentIndex(0), dec!(50));

let alerts = VecAlertHook::default();
tracker.check_limit(InstrumentIndex(0), dec!(20), &alerts);
assert!(!alerts.alerts.lock().unwrap().is_empty());
```

```rust
use jackbot_risk::volatility::VolatilityScaler;
use rust_decimal_macros::dec;

let scaler = VolatilityScaler::new(dec!(0.02), dec!(0.5), dec!(2));
let size = scaler.adjust_position(dec!(10), dec!(0.04));
assert_eq!(size, dec!(5));
```

