use crate::alert::{RiskAlertHook, RiskViolation};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::hash::Hash;
use jackbot_instrument::{exchange::ExchangeId, instrument::InstrumentIndex};

/// Tracks net position per (exchange, instrument) pair.
#[derive(Debug, Default, Clone)]
pub struct PositionTracker<InstrumentKey = InstrumentIndex> {
    positions: HashMap<(ExchangeId, InstrumentKey), Decimal>,
}

impl<InstrumentKey> PositionTracker<InstrumentKey>
where
    InstrumentKey: Eq + Hash + Clone,
{
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self { positions: HashMap::new() }
    }

    /// Update the position for the given key.
    /// Positive amounts represent long exposure, negative amounts short.
    pub fn update(&mut self, exchange: ExchangeId, instrument: InstrumentKey, qty: Decimal) {
        *self.positions.entry((exchange, instrument)).or_insert(Decimal::ZERO) += qty;
    }

    /// Current position for the given key.
    pub fn position(&self, exchange: ExchangeId, instrument: &InstrumentKey) -> Decimal {
        *self.positions.get(&(exchange, instrument.clone())).unwrap_or(&Decimal::ZERO)
    }

    /// Check a position limit and emit an alert via the provided hook if exceeded.
    pub fn check_limit(
        &self,
        exchange: ExchangeId,
        instrument: InstrumentKey,
        limit: Decimal,
        hook: &impl RiskAlertHook<(ExchangeId, InstrumentKey)>,
    ) {
        let pos = self.position(exchange, &instrument);
        if pos.abs() > limit {
            hook.alert(RiskViolation::ExposureLimit {
                instrument: (exchange, instrument),
                exposure: pos,
                limit,
            });
        }
    }
}
