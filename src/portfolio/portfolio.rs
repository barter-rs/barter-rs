use crate::data::market::MarketEvent;
use crate::strategy::signal::{SignalEvent, Decision, SignalStrength};
use crate::execution::fill::FillEvent;
use crate::portfolio::order::{OrderEvent, OrderType};
use crate::portfolio::error::PortfolioError;
use crate::portfolio::repository::redis::{PositionHandler, ValueHandler, CashHandler, determine_position_id};
use uuid::Uuid;
use crate::portfolio::risk::{DefaultRisk, OrderEvaluator};
use crate::portfolio::allocator::{DefaultAllocator, OrderAllocator};
use crate::portfolio::position::{PositionUpdater, PositionExiter, Position, PositionEnterer, Direction};
use chrono::Utc;
use crate::portfolio::error::PortfolioError::BuilderIncomplete;
use std::collections::HashMap;

/// Updates the Portfolio from an input [MarketEvent].
pub trait MarketUpdater {
    /// Determines if the Portfolio has any open [Position]s relating to the input [MarketEvent],
    /// and if so updates it using the market data.
    fn update_from_market(&mut self, market: &MarketEvent) -> Result<(), PortfolioError>;
}

/// May generate an [OrderEvent] from an input advisory [SignalEvent].
pub trait OrderGenerator {
    /// May generate an [OrderEvent] after analysing an input advisory [SignalEvent].
    fn generate_order(&mut self, signal: &SignalEvent) -> Result<Option<OrderEvent>, PortfolioError>;
}

/// Updates the Portfolio from an input [FillEvent].
pub trait FillUpdater {
    /// Updates the Portfolio state using the input [FillEvent]. The [FillEvent] triggers a
    /// [Position] entry or exit, and the Portfolio updates key fields such as current_cash and
    /// current_value accordingly.
    fn update_from_fill(&mut self, fill: &FillEvent) -> Result<(), PortfolioError>;
}

/// Components for construction a [PersistedMetaPortfolio] via the new() constructor method.
#[derive(Debug)]
pub struct Components {
    pub allocator: DefaultAllocator,
    pub risk: DefaultRisk,
    pub starting_cash: f64,
}

/// Portfolio with state persisted in a repository. Implements [MarketUpdater], [OrderGenerator],
/// and [FillUpdater]. The Portfolio analyses an advisory [SignalEvent] from a Strategy and decides
/// whether to place a corresponding [OrderEvent]. If a [Position] is opened, the Portfolio keeps
/// track the it's state, as well as it's own.
pub struct PersistedMetaPortfolio<T> where T: PositionHandler + ValueHandler + CashHandler {
    /// Unique ID for the [PersistedMetaPortfolio].
    id: Uuid,
    /// Repository for the [PersistedMetaPortfolio] to persist it's state in. Implements
    /// [PositionHandler], [ValueHandler], and [CashHandler].
    repository: T,
    /// Allocation manager implements [OrderAllocator].
    allocation_manager: DefaultAllocator,
    /// Risk manager implements [OrderEvaluator].
    risk_manager: DefaultRisk,
}

impl<T> MarketUpdater for PersistedMetaPortfolio<T> where T: PositionHandler + ValueHandler + CashHandler {
    fn update_from_market(&mut self, market: &MarketEvent) -> Result<(), PortfolioError> {
        // Determine the position_id & associated Option<Position> related to the input MarketEvent
        let position_id = determine_position_id(&self.id, &market.exchange, &market.symbol);

        // If Portfolio contains an open Position for the MarketEvent Symbol-Exchange combination
        if let Some(mut position) = self.repository.get_position(&position_id)? {

            // Update Position
            position.update(market);
        }

        Ok(())
    }
}

impl<T> OrderGenerator for PersistedMetaPortfolio<T> where T: PositionHandler + ValueHandler + CashHandler {
    fn generate_order(&mut self, signal: &SignalEvent) -> Result<Option<OrderEvent>, PortfolioError> {
        // Determine the position_id & associated Option<Position> related to input SignalEvent
        let position_id = determine_position_id(&self.id, &signal.exchange, &signal.symbol);

        let position = self.repository.get_position(&position_id)?;
        let position = position.as_ref();

        if self.no_cash_to_enter_new_position(&position)? {
            return Ok(None);
        }

        // Parse signals from Strategy to determine net signal & associated strength
        let (signal_decision, signal_strength) =
            match parse_signal_decisions(&position, &signal.signals) {
                None => return Ok(None),
                Some(net_signal) => net_signal,
            };

        let mut order = OrderEvent::builder()
            .trace_id(signal.trace_id)
            .timestamp(Utc::now())
            .exchange(signal.exchange.clone())
            .symbol(signal.symbol.clone())
            .close(signal.close)
            .decision(signal_decision.clone())
            .quantity(0.0)
            .order_type(OrderType::default())
            .build()?;

        // OrderEvent size allocation
        order = self
            .allocation_manager
            .allocate_order(order, position, *signal_strength)?;

        // OrderEvent risk evaluation - refine or cancel
        Ok(self.risk_manager.evaluate_order(order)?)
    }
}

impl<T> FillUpdater for PersistedMetaPortfolio<T> where T: PositionHandler + ValueHandler + CashHandler {
    fn update_from_fill(&mut self, fill: &FillEvent) -> Result<(), PortfolioError> {
        // Get the Portfolio value & cash from repository
        let mut current_value = self.repository.get_current_value(&self.id)?;
        let mut current_cash = self.repository.get_current_cash(&self.id)?;

        // Determine the position_id that is related to the input FillEvent
        let position_id = determine_position_id(&self.id, &fill.exchange, &fill.symbol);

        // EXIT SCENARIO - FillEvent for Symbol-Exchange with open Position
        if let Some(mut position) = self.repository.remove_position(&position_id)? {

            // Exit Position
            position.exit(fill)?;

            // Update Portfolio cash & value on exit
            current_cash += position.exit_value_gross - position.exit_fees_total;
            current_value += position.result_profit_loss;
        }

        // ENTRY SCENARIO - FillEvent for Symbol-Exchange with no Position
        else {
            let position = Position::enter(&fill)?;

            // Update Portfolio cash entry
            current_cash += -position.enter_value_gross - position.enter_fees_total;

            // Add to current Positions in repository
            self.repository.set_position(&self.id, &position)?;
        }

        // Persist updated Portfolio value & cash in repository
        self.repository.set_current_value(&self.id, current_value)?;
        self.repository.set_current_cash(&self.id, current_cash)?;

        Ok(())
    }
}

impl<T> PersistedMetaPortfolio<T> where T: PositionHandler + ValueHandler + CashHandler {
    /// Constructs a new [PersistedMetaPortfolio] component using the provided [Components] struct.
    pub fn new(components: Components, repository: T) -> Self {
        PersistedMetaPortfolio::builder()
            .id(Uuid::new_v4())
            .repository(repository)
            .allocation_manager(components.allocator)
            .risk_manager(components.risk)
            .build()
            .expect("Failed to build a PersistedMetaPortfolio")
    }

    /// Returns a [PersistedMetaPortfolio] instance.
    pub fn builder() -> PersistedMetaPortfolioBuilder<T> {
        PersistedMetaPortfolioBuilder::new()
    }

    /// Determines if the Portfolio has any cash to enter a new [Position].
    fn no_cash_to_enter_new_position(&mut self, position: &Option<&Position>) -> Result<bool, PortfolioError> {
        let current_cash = self.repository.get_current_cash(&self.id)?;
        Ok(position.is_none() && current_cash == 0.0)
    }
}

/// Builder to construct [PersistedMetaPortfolio] instances.
pub struct PersistedMetaPortfolioBuilder<T> where T: PositionHandler + ValueHandler + CashHandler {
    id: Option<Uuid>,
    repository: Option<T>,
    allocation_manager: Option<DefaultAllocator>,
    risk_manager: Option<DefaultRisk>,
}

impl<T> PersistedMetaPortfolioBuilder<T> where T: PositionHandler + ValueHandler + CashHandler {
    pub fn new() -> Self {
        Self {
            id: None,
            repository: None,
            allocation_manager: None,
            risk_manager: None,
        }
    }

    pub fn id(mut self, value: Uuid) -> Self {
        self.id = Some(value);
        self
    }

    pub fn repository(mut self, value: T) -> Self {
        self.repository = Some(value);
        self
    }

    pub fn allocation_manager(mut self, value: DefaultAllocator) -> Self {
        self.allocation_manager = Some(value);
        self
    }

    pub fn risk_manager(mut self, value: DefaultRisk) -> Self {
        self.risk_manager = Some(value);
        self
    }

    pub fn build(self) -> Result<PersistedMetaPortfolio<T>, PortfolioError> {
        if let (
            Some(id),
            Some(repository),
            Some(allocation_manager),
            Some(risk_manager)
        ) = (
            self.id,
            self.repository,
            self.allocation_manager,
            self.risk_manager,
        ) {
            Ok(PersistedMetaPortfolio {
                id,
                repository,
                allocation_manager,
                risk_manager,
            })
        } else {
            Err(BuilderIncomplete())
        }
    }
}

/// Parses an incoming [SignalEvent]'s signals map. Determines what the net signal [Decision] will
/// be, and it's associated [SignalStrength].
pub fn parse_signal_decisions<'a>(position: &'a Option<&Position>, signals: &'a HashMap<Decision, SignalStrength>) -> Option<(&'a Decision, &'a SignalStrength)> {
    let signal_close_long = signals.get_key_value(&Decision::CloseLong);
    let signal_long = signals.get_key_value(&Decision::Long);
    let signal_close_short = signals.get_key_value(&Decision::CloseShort);
    let signal_short = signals.get_key_value(&Decision::Short);

    if let Some(position) = position {
        match position.direction {
            Direction::Long => {
                if signal_close_long.is_some() {
                    return signal_close_long;
                }
            }
            Direction::Short => {
                if signal_close_short.is_some() {
                    return signal_close_short;
                }
            }
        }
    }

    if signal_long.is_some() && signal_short.is_some() {
        return None;
    }
    if signal_long.is_some() {
        return signal_long;
    }
    if signal_short.is_some() {
        return signal_short;
    }

    return None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_signal_decisions_to_net_close_long() {
        // Some(Position)
        let mut position = Position::default();
        position.direction = Direction::Long;
        let position = Some(position);
        let position = position.as_ref();

        // Signals HashMap
        let mut signals = HashMap::with_capacity(4);
        signals.insert(Decision::CloseLong, 1.0);
        signals.insert(Decision::Short, 1.0);

        let actual = parse_signal_decisions(&position, &signals);

        assert_eq!(actual.unwrap().0, &Decision::CloseLong);
    }

    #[test]
    fn parse_signal_decisions_to_net_close_long_with_conflicting_signals() {
        // Some(Position)
        let mut position = Position::default();
        position.direction = Direction::Long;
        let position = Some(position);
        let position = position.as_ref();

        // Signals HashMap
        let mut signals = HashMap::with_capacity(4);
        signals.insert(Decision::CloseLong, 1.0);
        signals.insert(Decision::CloseShort, 1.0);
        signals.insert(Decision::Short, 1.0);
        signals.insert(Decision::Long, 1.0);

        let actual = parse_signal_decisions(&position, &signals);

        assert_eq!(actual.unwrap().0, &Decision::CloseLong);
    }

    #[test]
    fn parse_signal_decisions_to_net_close_short() {
        // Some(Position)
        let mut position = Position::default();
        position.direction = Direction::Short;
        let position = Some(position);
        let position = position.as_ref();

        // Signals HashMap
        let mut signals = HashMap::with_capacity(4);
        signals.insert(Decision::CloseShort, 1.0);
        signals.insert(Decision::Long, 1.0);

        let actual = parse_signal_decisions(&position, &signals);

        assert_eq!(actual.unwrap().0, &Decision::CloseShort);
    }

    #[test]
    fn parse_signal_decisions_to_net_close_short_with_conflicting_signals() {
        // Some(Position)
        let mut position = Position::default();
        position.direction = Direction::Short;
        let position = Some(position);
        let position = position.as_ref();

        // Signals HashMap
        let mut signals = HashMap::with_capacity(4);
        signals.insert(Decision::CloseShort, 1.0);
        signals.insert(Decision::CloseLong, 1.0);
        signals.insert(Decision::Short, 1.0);
        signals.insert(Decision::Long, 1.0);

        let actual = parse_signal_decisions(&position, &signals);

        assert_eq!(actual.unwrap().0, &Decision::CloseShort);
    }

    #[test]
    fn parse_signal_decisions_to_net_long() {
        let position = None;
        let position = position.as_ref();

        // Signals HashMap
        let mut signals = HashMap::with_capacity(4);
        signals.insert(Decision::Long, 1.0);
        signals.insert(Decision::CloseShort, 1.0);

        let actual = parse_signal_decisions(&position, &signals);

        assert_eq!(actual.unwrap().0, &Decision::Long);
    }

    #[test]
    fn parse_signal_decisions_to_net_short() {
        let position = None;
        let position = position.as_ref();

        // Signals HashMap
        let mut signals = HashMap::with_capacity(4);
        signals.insert(Decision::Short, 1.0);
        signals.insert(Decision::CloseLong, 1.0);

        let actual = parse_signal_decisions(&position, &signals);

        assert_eq!(actual.unwrap().0, &Decision::Short);
    }

    #[test]
    fn parse_signal_decisions_to_none_with_conflicting_signals() {
        let position = None;
        let position = position.as_ref();

        // Signals HashMap
        let mut signals = HashMap::with_capacity(4);
        signals.insert(Decision::Long, 1.0);
        signals.insert(Decision::CloseShort, 1.0);
        signals.insert(Decision::Short, 1.0);
        signals.insert(Decision::CloseLong, 1.0);

        let actual = parse_signal_decisions(&position, &signals);

        assert_eq!(actual, None);
    }
}