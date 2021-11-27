use crate::data::market::MarketEvent;
use crate::execution::fill::FillEvent;
use crate::portfolio::error::PortfolioError;
use crate::portfolio::order::OrderEvent;
use crate::portfolio::position::Position;
use crate::strategy::signal::SignalEvent;

pub mod allocator;
pub mod error;
pub mod order;
pub mod portfolio;
pub mod position;
pub mod repository;
pub mod risk;

/// Updates the Portfolio from an input [MarketEvent].
pub trait MarketUpdater {
    /// Determines if the Portfolio has an open Position relating to the input [MarketEvent],
    /// and if so updates it using the market data.
    fn update_from_market(&mut self, market: &MarketEvent) -> Result<(), PortfolioError>;
}

/// May generate an [OrderEvent] from an input advisory [SignalEvent].
pub trait OrderGenerator {
    /// May generate an [OrderEvent] after analysing an input advisory [SignalEvent].
    fn generate_order(
        &mut self,
        signal: &SignalEvent,
    ) -> Result<Option<OrderEvent>, PortfolioError>;
}

/// Updates the Portfolio from an input [FillEvent].
pub trait FillUpdater {
    /// Updates the Portfolio state using the input [FillEvent]. The [FillEvent] triggers a
    /// Position entry or exit, and the Portfolio updates key fields such as current_cash and
    /// current_value accordingly.
    fn update_from_fill(&mut self, fill: &FillEvent) -> Result<Option<Position>, PortfolioError>;
}
