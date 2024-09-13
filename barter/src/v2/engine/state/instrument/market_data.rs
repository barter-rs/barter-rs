use barter_data::subscription::book::OrderBookL1;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Default, Deserialize, Serialize, Constructor)]
pub struct MarketState {
    pub l1: OrderBookL1,
}

impl MarketState {
    pub fn update_from_l1(&mut self, l1: OrderBookL1) {
        let _ = std::mem::replace(&mut self.l1, l1);
    }
}
