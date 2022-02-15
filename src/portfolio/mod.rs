use crate::data::market::MarketEvent;
use crate::event::Event;
use crate::execution::fill::FillEvent;
use crate::portfolio::error::PortfolioError;
use crate::portfolio::order::OrderEvent;
use crate::portfolio::position::PositionUpdate;
use crate::strategy::signal::{SignalEvent, SignalForceExit};
use serde::Serialize;

pub mod allocator;
pub mod error;
pub mod order;
pub mod portfolio;
pub mod position;
pub mod repository;
pub mod risk;

/// Updates the Portfolio from an input [`MarketEvent`].
pub trait MarketUpdater {
    /// Determines if the Portfolio has an open Position relating to the input [`MarketEvent`]. If
    /// so it updates it using the market data, and returns a [`PositionUpdate`] detailing the
    /// changes.
    fn update_from_market(
        &mut self,
        market: &MarketEvent,
    ) -> Result<Option<PositionUpdate>, PortfolioError>;
}

/// May generate an [`OrderEvent`] from an input advisory [`SignalEvent`].
pub trait OrderGenerator {
    /// May generate an [`OrderEvent`] after analysing an input advisory [`SignalEvent`].
    fn generate_order(
        &mut self,
        signal: &SignalEvent,
    ) -> Result<Option<OrderEvent>, PortfolioError>;

    /// Generates an exit [`OrderEvent`] if there is an open [`Position`] associated with the
    /// input [`SignalForceExit`]'s [`PositionId`].
    fn generate_exit_order(
        &mut self,
        signal: SignalForceExit,
    ) -> Result<Option<OrderEvent>, PortfolioError>;
}

/// Updates the Portfolio from an input [`FillEvent`].
pub trait FillUpdater<Statistic: Serialize> {
    /// Updates the Portfolio state using the input [`FillEvent`]. The [`FillEvent`] triggers a
    /// Position entry or exit, and the Portfolio updates key fields such as current_cash and
    /// current_value accordingly.
    fn update_from_fill(
        &mut self,
        fill: &FillEvent,
    ) -> Result<Vec<Event<Statistic>>, PortfolioError>;
}
