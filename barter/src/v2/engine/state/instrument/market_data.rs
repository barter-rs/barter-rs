use barter_data::subscription::book::OrderBookL1;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use barter_data::event::{DataKind, MarketEvent};
use crate::v2::Snapshot;

pub trait MarketDataManager<InstrumentKey>: Clone {
    fn update_from_snapshot(&mut self, snapshot: Snapshot<Self>);
    fn update_from_market(&mut self, event: &MarketEvent<InstrumentKey, DataKind>);
}

#[derive(Debug, Copy, Clone, PartialEq, Default, Deserialize, Serialize, Constructor)]
pub struct MarketState {
    pub l1: OrderBookL1,
}

impl MarketState {
    pub fn update_from_l1(&mut self, l1: OrderBookL1) {
        let _ = std::mem::replace(&mut self.l1, l1);
    }
}
