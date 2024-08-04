use barter_data::subscription::book::OrderBookL1;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct MarketState {
    pub l1: OrderBookL1,
}

impl Default for MarketState {
    fn default() -> Self {
        Self {
            l1: OrderBookL1::default(),
        }
    }
}

impl MarketState {
    pub fn update_from_l1(&mut self, l1: OrderBookL1) {
        let _ = std::mem::replace(&mut self.l1, l1);
    }
}
