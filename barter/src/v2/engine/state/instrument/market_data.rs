use crate::v2::engine::Processor;
use barter_data::{
    event::{DataKind, MarketEvent},
    subscription::book::OrderBookL1,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub trait MarketDataState<InstrumentKey>
where
    Self: Debug + Clone + Send + for<'a> Processor<&'a MarketEvent<InstrumentKey, Self::EventKind>>,
{
    type EventKind: Debug + Clone + Send;
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct DefaultMarketData {
    pub l1: OrderBookL1,
    pub last_traded_price: f64,
}

impl<InstrumentKey> MarketDataState<InstrumentKey> for DefaultMarketData {
    type EventKind = DataKind;
}

impl<InstrumentKey> Processor<&MarketEvent<InstrumentKey, DataKind>> for DefaultMarketData {
    type Output = ();

    fn process(&mut self, event: &MarketEvent<InstrumentKey, DataKind>) -> Self::Output {
        match &event.kind {
            DataKind::Trade(trade) => {
                self.last_traded_price = trade.price;
            }
            DataKind::OrderBookL1(l1) => {
                self.l1 = l1.clone();
            }
            _ => {}
        }
    }
}
