use rust_decimal::Decimal;
use crate::smart_trade::SmartTradeSignal;

#[derive(Debug, Clone)]
pub struct MultiLevelStop {
    levels: Vec<Decimal>,
    current: usize,
}

impl MultiLevelStop {
    pub fn new(mut levels: Vec<Decimal>) -> Self {
        levels.sort_by(|a, b| b.cmp(a));
        Self { levels, current: 0 }
    }

    pub fn update(&mut self, price: Decimal) -> Option<SmartTradeSignal> {
        if self.current >= self.levels.len() {
            return None;
        }
        while self.current < self.levels.len() && price <= self.levels[self.current] {
            let idx = self.current;
            self.current += 1;
            return Some(SmartTradeSignal::StopLevel(idx, price));
        }
        None
    }
}
