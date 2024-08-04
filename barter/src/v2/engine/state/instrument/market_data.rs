use crate::v2::engine::Processor;
use barter_data::{books::OrderBook, event::MarketEvent, subscription::book::OrderBookL1};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub trait MarketDataManager<InstrumentKey>
where
    Self: for<'a> Processor<&'a MarketEvent<InstrumentKey, Self::MarketEventKind>>,
{
    type Snapshot: Clone;
    type MarketEventKind: Debug;
}

// pub trait MarketDataManager<InstrumentKey>
// where
//     Self: UpdateFromSnapshot<Self::Snapshot>,
// {
//     type Snapshot;
//     type MarketDataKind: Debug;
//     fn update_from_market(&mut self, event: &MarketEvent<InstrumentKey, Self::MarketDataKind>);
// }

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct DefaultMarketState {
    pub l1: OrderBookL1,
    pub l2: OrderBook,
}

// impl UpdateFromSnapshot<DefaultMarketState> for DefaultMarketState {
//     fn update_from_snapshot(&mut self, snapshot: &DefaultMarketState) {
//         *self = snapshot.clone();
//     }
// }
//
// impl<InstrumentKey> MarketState<InstrumentKey> for DefaultMarketState {
//     type Snapshot = Self;
//     type MarketDataKind = DataKind;
//
//     fn update_from_market(&mut self, event: &MarketEvent<InstrumentKey, Self::MarketDataKind>) {
//         match &event.kind {
//             DataKind::OrderBookL1(l1) => *self.l1 = l1,
//             DataKind::OrderBook(book) => *self.l2 = book.clone(),
//             _ => {}
//         }
//     }
// }
