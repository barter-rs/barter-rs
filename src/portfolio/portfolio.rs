use crate::data::market::MarketEvent;
use crate::strategy::signal::SignalEvent;
use crate::execution::fill::FillEvent;
use crate::portfolio::order::OrderEvent;
use crate::portfolio::error::PortfolioError;

pub trait MarketUpdater {
    fn update_from_market(&mut self, market: &MarketEvent) -> Result<(), PortfolioError>;
}

pub trait OrderGenerator {
    fn generate_order(&mut self, signal: &SignalEvent) -> Result<Option<OrderEvent>, PortfolioError>;
}

pub trait FillUpdater {
    fn update_from_fill(&mut self, fill: &FillEvent) -> Result<(), PortfolioError>;
}