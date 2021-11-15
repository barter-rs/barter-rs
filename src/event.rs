use crate::data::market::MarketEvent;
use crate::execution::fill::FillEvent;
use crate::portfolio::order::OrderEvent;
use crate::strategy::signal::SignalEvent;

#[derive(Debug)]
pub enum Event {
    Market(MarketEvent),
    Signal(SignalEvent),
    Order(OrderEvent),
    Fill(FillEvent),
}
