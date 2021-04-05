use crate::data::market::MarketEvent;
use crate::strategy::signal::SignalEvent;
use crate::portfolio::order::OrderEvent;
use crate::execution::fill::FillEvent;

pub enum Event {
    Market(MarketEvent),
    Signal(SignalEvent),
    Order(OrderEvent),
    Fill(FillEvent),
}