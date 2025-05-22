use crate::alert::{RiskAlertHook, RiskViolation};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::hash::Hash;
use jackbot_instrument::instrument::InstrumentIndex;

/// Manages correlation limits between instrument pairs.
#[derive(Debug, Default, Clone)]
pub struct CorrelationMatrix<InstrumentKey = InstrumentIndex> {
    limits: HashMap<(InstrumentKey, InstrumentKey), Decimal>,
}

impl<InstrumentKey> CorrelationMatrix<InstrumentKey>
where
    InstrumentKey: Eq + Hash + Clone,
{
    pub fn new() -> Self { Self { limits: HashMap::new() } }

    pub fn set_limit(&mut self, a: InstrumentKey, b: InstrumentKey, limit: Decimal) {
        self.limits.insert((a, b), limit);
    }

    pub fn check_limit(&self, a: InstrumentKey, b: InstrumentKey, exposure: Decimal, hook: &impl RiskAlertHook<InstrumentKey>) {
        if let Some(limit) = self.limits.get(&(a.clone(), b.clone())) {
            if exposure > *limit {
                hook.alert(RiskViolation::CorrelationLimit { instruments: (a, b), combined_exposure: exposure, limit: *limit });
            }
        }
    }
}
