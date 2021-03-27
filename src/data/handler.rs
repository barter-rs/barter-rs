use crate::error::BarterError;
use crate::data::market::MarketEvent;

pub trait Continuer {
    fn should_continue(&self) -> bool;
}

pub trait MarketGenerator {
    fn generate_market(&mut self) -> Result<MarketEvent, BarterError>;
}