use crate::alert::{RiskAlertHook, RiskViolation};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::hash::Hash;
use jackbot_instrument::instrument::InstrumentIndex;

/// Tracks exposure per instrument.
#[derive(Debug, Default, Clone)]
pub struct ExposureTracker<InstrumentKey = InstrumentIndex> {
    exposures: HashMap<InstrumentKey, Decimal>,
}

impl<InstrumentKey> ExposureTracker<InstrumentKey>
where
    InstrumentKey: Eq + Hash + Clone,
{
    pub fn new() -> Self {
        Self { exposures: HashMap::new() }
    }

    pub fn update(&mut self, instrument: InstrumentKey, notional: Decimal) {
        *self.exposures.entry(instrument).or_insert(Decimal::ZERO) += notional;
    }

    pub fn exposure(&self, instrument: &InstrumentKey) -> Decimal {
        *self.exposures.get(instrument).unwrap_or(&Decimal::ZERO)
    }

    pub fn check_limit(&self, instrument: InstrumentKey, limit: Decimal, hook: &impl RiskAlertHook<InstrumentKey>) {
        let exposure = self.exposure(&instrument);
        if exposure > limit {
            hook.alert(RiskViolation::ExposureLimit { instrument, exposure, limit });
        }
    }
}
