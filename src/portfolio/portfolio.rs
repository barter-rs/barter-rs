use crate::data::market::MarketEvent;
use crate::strategy::signal::{SignalEvent, Decision, SignalStrength};
use crate::execution::fill::FillEvent;
use crate::portfolio::order::{OrderEvent, OrderType};
use crate::portfolio::error::PortfolioError;
use crate::portfolio::repository::redis::{PositionHandler, ValueHandler, CashHandler, determine_position_id};
use uuid::Uuid;
use crate::portfolio::risk::{DefaultRisk, OrderEvaluator};
use crate::portfolio::allocator::{DefaultAllocator, OrderAllocator};
use crate::portfolio::position::{PositionUpdater, PositionExiter, Position, Fee, PositionEnterer, Direction};
use chrono::Utc;
use crate::portfolio::error::PortfolioError::BuilderIncomplete;
use std::collections::HashMap;

pub trait MarketUpdater {
    fn update_from_market(&mut self, market: &MarketEvent) -> Result<(), PortfolioError>;
}

pub trait OrderGenerator {
    fn generate_order(&mut self, signal: &SignalEvent) -> Result<Option<OrderEvent>, PortfolioError>;
}

pub trait FillUpdater {
    fn update_from_fill(&mut self, fill: &FillEvent) -> Result<(), PortfolioError>;
}

#[derive(Debug)]
pub struct Components {
    pub allocator: DefaultAllocator,
    pub risk: DefaultRisk,
    pub starting_cash: f64,
}

pub struct PersistedMetaPortfolio<T> where T: PositionHandler + ValueHandler + CashHandler {
    id: Uuid,
    repository: T,
    allocation_manager: DefaultAllocator,
    risk_manager: DefaultRisk,
}

// Todo: Check over portfolio trait impls
//  - extract testable methods out of the above methods to make it easier to test & read
//  - Make EntryTotalFees it's own field, with a dirty bag for each individual fee
//  - Swap exit and entry position in this function around - seems more clear

impl<T> MarketUpdater for PersistedMetaPortfolio<T> where T: PositionHandler + ValueHandler + CashHandler {
    fn update_from_market(&mut self, market: &MarketEvent) -> Result<(), PortfolioError> {
        // Determine the position_id & associated Option<Position> related to the input MarketEvent
        let position_id = determine_position_id(&self.id, &market.exchange, &market.symbol);

        // If Portfolio contains an open Position for the MarketEvent Symbol-Exchange combination
        if let Some(mut position) = self.repository.get_position(&position_id)? {

            // Update Position
            position.update(market)?;
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
        let net_signal: (&Decision, &SignalStrength) =
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
            .decision(net_signal.0.clone())
            .quantity(0.0)
            .order_type(OrderType::default())
            .build()?;

        // OrderEvent size allocation
        order = self
            .allocation_manager
            .allocate_order(order, position, *net_signal.1)?;

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

            // Update Portfolio cash & value on exit --> use result_profit_loss
            let total_exit_fees = position.exit_fees.get(&Fee::TotalFees)
                .expect("Exited Position contains None for exit_fees instead of Some");

            current_cash += position.exit_value_gross - total_exit_fees;
            current_value += position.result_profit_loss;

        }

        // ENTRY SCENARIO - FillEvent for Symbol-Exchange with no Position
        else {
            let position = Position::enter(&fill)?;

            // Update Portfolio cash entry
            let total_enter_fees = position.enter_fees.get(&Fee::TotalFees)
                .expect("Entered Position contains None enter_fees instead of Some");

            current_cash += -position.enter_value_gross - total_enter_fees;

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
    pub fn new(components: Components, repository: T) -> Self {
        PersistedMetaPortfolio::builder()
            .id(Uuid::new_v4())
            .repository(repository)
            .allocation_manager(components.allocator)
            .risk_manager(components.risk)
            .build()
            .expect("Failed to build a PersistedMetaPortfolio")
    }

    pub fn builder() -> PersistedMetaPortfolioBuilder<T> {
        PersistedMetaPortfolioBuilder::new()
    }

    fn no_cash_to_enter_new_position(&mut self, position: &Option<&Position>) -> Result<bool, PortfolioError> {
        let current_cash = self.repository.get_current_cash(&self.id)?;
        Ok(position.is_none() && current_cash == 0.0)
    }
}

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

pub fn parse_signal_decisions<'a>(position: &'a Option<&Position>, signal_pairs: &'a HashMap<Decision, SignalStrength>) -> Option<(&'a Decision, &'a SignalStrength)> {
    let signal_close_long = signal_pairs.get_key_value(&Decision::Long);
    let signal_long = signal_pairs.get_key_value(&Decision::Long);
    let signal_close_short = signal_pairs.get_key_value(&Decision::Long);
    let signal_short = signal_pairs.get_key_value(&Decision::Long);

    match position.is_some() {
        true => {
            match position.unwrap().direction {
                Direction::Long => {
                    if signal_close_long.is_some() {
                        return signal_close_long;
                    }
                },
                Direction::Short => {
                    if signal_close_short.is_some() {
                        return signal_close_short;
                    }
                }
            }
        }
        false => {
            if signal_long.is_some() {
                return signal_long;
            }
            else if signal_short.is_some() {
                return signal_short;
            }
        }
    }
    return None;
}