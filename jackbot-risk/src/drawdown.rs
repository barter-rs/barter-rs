use crate::alert::{RiskAlertHook, RiskViolation};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::hash::Hash;
use jackbot_instrument::instrument::InstrumentIndex;

/// Tracks realised/unrealised PnL to compute drawdown percentages.
#[derive(Debug, Default, Clone)]
pub struct DrawdownTracker<InstrumentKey = InstrumentIndex> {
    peak: HashMap<InstrumentKey, Decimal>,
    current: HashMap<InstrumentKey, Decimal>,
}

impl<InstrumentKey> DrawdownTracker<InstrumentKey>
where
    InstrumentKey: Eq + Hash + Clone,
{
    pub fn new() -> Self {
        Self { peak: HashMap::new(), current: HashMap::new() }
    }

    pub fn update_pnl(&mut self, instrument: InstrumentKey, pnl: Decimal) {
        let cur = self.current.entry(instrument.clone()).or_insert(Decimal::ZERO);
        *cur += pnl;
        let peak = self.peak.entry(instrument).or_insert(*cur);
        if *cur > *peak {
            *peak = *cur;
        }
    }

    pub fn drawdown(&self, instrument: &InstrumentKey) -> Decimal {
        let cur = *self.current.get(instrument).unwrap_or(&Decimal::ZERO);
        let peak = *self.peak.get(instrument).unwrap_or(&cur);
        if peak.is_zero() { Decimal::ZERO } else { (peak - cur) / peak }
    }

    pub fn check_limit(&self, instrument: InstrumentKey, limit: Decimal, hook: &impl RiskAlertHook<InstrumentKey>) {
        let dd = self.drawdown(&instrument);
        if dd > limit {
            hook.alert(RiskViolation::DrawdownLimit { instrument, drawdown: dd, limit });
        }
    }
}
