use rust_decimal::Decimal;
use crate::smart_trade::SmartTradeSignal;

#[derive(Debug, Clone)]
pub struct TrailingTakeProfit {
    trailing: Decimal,
    highest: Option<Decimal>,
    triggered: bool,
}

impl TrailingTakeProfit {
    pub fn new(trailing: Decimal) -> Self {
        Self { trailing, highest: None, triggered: false }
    }

    pub fn update(&mut self, price: Decimal) -> Option<SmartTradeSignal> {
        if self.triggered {
            return None;
        }
        match self.highest {
            Some(high) => {
                if price > high {
                    self.highest = Some(price);
                }
                if price <= high - self.trailing {
                    self.triggered = true;
                    return Some(SmartTradeSignal::TakeProfit(price));
                }
            }
            None => self.highest = Some(price),
        }
        None
    }
}
