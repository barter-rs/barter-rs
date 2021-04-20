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

/// Components for construction a [MetaPortfolio] via the new() constructor method.
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
pub struct MetaPortfolio<T> where T: PositionHandler + ValueHandler + CashHandler {
    /// Unique ID for the [MetaPortfolio].
    pub id: Uuid,
    /// Starting cash for the Portfolio, used to persist the initial state to the repository.
    pub starting_cash: f64,
    /// Repository for the [MetaPortfolio] to persist it's state in. Implements
    /// [PositionHandler], [ValueHandler], and [CashHandler].
    pub repository: T,
    /// Allocation manager implements [OrderAllocator].
    allocation_manager: DefaultAllocator,
    /// Risk manager implements [OrderEvaluator].
    risk_manager: DefaultRisk,
}

impl<T> MarketUpdater for MetaPortfolio<T> where T: PositionHandler + ValueHandler + CashHandler {
    fn update_from_market(&mut self, market: &MarketEvent) -> Result<(), PortfolioError> {
        // Determine the position_id & associated Option<Position> related to the input MarketEvent
        let position_id = determine_position_id(&self.id, &market.exchange, &market.symbol);

        // If Portfolio contains an open Position for the MarketEvent Symbol-Exchange combination
        if let Some(mut position) = self.repository.get_position(&position_id)? {

            // Update Position
            position.update(market);
            self.repository.set_position(&self.id, &position)?;
        }

        Ok(())
    }
}

impl<T> OrderGenerator for MetaPortfolio<T> where T: PositionHandler + ValueHandler + CashHandler {
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

        let mut order = OrderEvent {
            event_type: OrderEvent::EVENT_TYPE,
            trace_id: signal.trace_id,
            timestamp: Utc::now(),
            exchange: signal.exchange.clone(),
            symbol: signal.symbol.clone(),
            close: signal.close,
            decision: signal_decision.clone(),
            quantity: 0.0,
            order_type: OrderType::default()
        };

        // OrderEvent size allocation
        order = self
            .allocation_manager
            .allocate_order(order, position, *signal_strength)?;

        // OrderEvent risk evaluation - refine or cancel
        Ok(self.risk_manager.evaluate_order(order)?)
    }
}

impl<T> FillUpdater for MetaPortfolio<T> where T: PositionHandler + ValueHandler + CashHandler {
    fn update_from_fill(&mut self, fill: &FillEvent) -> Result<(), PortfolioError> {

        // Get the Portfolio Cash from repository
        let mut current_cash = self.repository.get_current_cash(&self.id)?;

        // Determine the position_id that is related to the input FillEvent
        let position_id = determine_position_id(&self.id, &fill.exchange, &fill.symbol);

        // EXIT SCENARIO - FillEvent for Symbol-Exchange with open Position
        if let Some(mut position) = self.repository.remove_position(&position_id)? {

            // Exit Position & persist in repository closed_positions
            position.exit(fill)?;
            self.repository.set_closed_position(&self.id, &position)?;

            // Update Portfolio cash on exit - + enter_total_fees since included in result PnL
            current_cash += position.enter_value_gross + position.result_profit_loss + position.enter_fees_total;

            // Update Portfolio cash on exit & persist in repository
            let mut current_value = self.repository.get_current_value(&self.id)?;
            current_value += position.result_profit_loss;
            self.repository.set_current_value(&self.id, current_value)?;
        }

        // ENTRY SCENARIO - FillEvent for Symbol-Exchange with no Position
        else {
            let position = Position::enter(&fill)?;

            // Update Portfolio cash entry
            current_cash += -position.enter_value_gross - position.enter_fees_total;

            // Add to current Positions in repository
            self.repository.set_position(&self.id, &position)?;
        }

        // Persist updated Portfolio cash in repository
        self.repository.set_current_cash(&self.id, current_cash)?;

        Ok(())
    }
}

impl<T> MetaPortfolio<T> where T: PositionHandler + ValueHandler + CashHandler {
    /// Constructs a new [MetaPortfolio] component using the provided [Components] struct.
    pub fn new(components: Components, repository: T) -> Self {
        Self {
            id: Uuid::new_v4(),
            starting_cash: components.starting_cash,
            repository,
            allocation_manager: components.allocator,
            risk_manager: components.risk,
        }
    }

    /// Returns a [MetaPortfolio] instance.
    pub fn builder() -> MetaPortfolioBuilder<T> {
        MetaPortfolioBuilder::new()
    }

    /// Persist the initial [MetaPortfolio] state in the Repository.
    pub fn initialise(&mut self) -> Result<(), PortfolioError> {
        self.repository.set_current_cash(&self.id, self.starting_cash)?;
        self.repository.set_current_value(&self.id, self.starting_cash)?;
        Ok(())
    }

    /// Determines if the Portfolio has any cash to enter a new [Position].
    fn no_cash_to_enter_new_position(&mut self, position: &Option<&Position>) -> Result<bool, PortfolioError> {
        let current_cash = self.repository.get_current_cash(&self.id)?;
        Ok(position.is_none() && current_cash == 0.0)
    }
}

/// Builder to construct [MetaPortfolio] instances.
pub struct MetaPortfolioBuilder<T> where T: PositionHandler + ValueHandler + CashHandler {
    id: Option<Uuid>,
    starting_cash: Option<f64>,
    repository: Option<T>,
    allocation_manager: Option<DefaultAllocator>,
    risk_manager: Option<DefaultRisk>,
}

impl<T> MetaPortfolioBuilder<T> where T: PositionHandler + ValueHandler + CashHandler {
    pub fn new() -> Self {
        Self {
            id: None,
            starting_cash: None,
            repository: None,
            allocation_manager: None,
            risk_manager: None,
        }
    }

    pub fn id(mut self, value: Uuid) -> Self {
        self.id = Some(value);
        self
    }

    pub fn starting_cash(mut self, value: f64) -> Self {
        self.starting_cash = Some(value);
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

    pub fn build(self) -> Result<MetaPortfolio<T>, PortfolioError> {
        if let (
            Some(id),
            Some(starting_cash),
            Some(repository),
            Some(allocation_manager),
            Some(risk_manager)
        ) = (
            self.id,
            self.starting_cash,
            self.repository,
            self.allocation_manager,
            self.risk_manager,
        ) {
            Ok(MetaPortfolio {
                id,
                starting_cash,
                repository,
                allocation_manager,
                risk_manager,
            })
        } else {
            Err(PortfolioError::BuilderIncomplete)
        }
    }

    pub fn build_and_init(self) -> Result<MetaPortfolio<T>, PortfolioError> {
        if let (
            Some(id),
            Some(starting_cash),
            Some(repository),
            Some(allocation_manager),
            Some(risk_manager)
        ) = (
            self.id,
            self.starting_cash,
            self.repository,
            self.allocation_manager,
            self.risk_manager,
        ) {
            let mut portfolio = MetaPortfolio {
                id,
                starting_cash,
                repository,
                allocation_manager,
                risk_manager,
            };
            portfolio.initialise()?;
            Ok(portfolio)
        } else {
            Err(PortfolioError::BuilderIncomplete)
        }
    }
}

/// Parses an incoming [SignalEvent]'s signals map. Determines what the net signal [Decision] will
/// be, and it's associated [SignalStrength].
pub fn parse_signal_decisions<'a>(position: &'a Option<&Position>, signals: &'a HashMap<Decision, SignalStrength>) -> Option<(&'a Decision, &'a SignalStrength)> {
    // Determine the presence of signals in the provided signals HashMap
    let signal_close_long = signals.get_key_value(&Decision::CloseLong);
    let signal_long = signals.get_key_value(&Decision::Long);
    let signal_close_short = signals.get_key_value(&Decision::CloseShort);
    let signal_short = signals.get_key_value(&Decision::Short);

    // If an existing Position exists, check for net close signals
    if let Some(position) = position {
        return match position.direction {
            Direction::Long if signal_close_long.is_some() => signal_close_long,
            Direction::Short if signal_close_short.is_some() => signal_close_short,
            _ => None,
        }
    }

    // Else check for net open signals
    match (signal_long, signal_short) {
        (Some(signal_long), None) => Some(signal_long),
        (None, Some(signal_short)) => Some(signal_short),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio::repository::error::RepositoryError;
    use crate::portfolio::position::PositionBuilder;
    use crate::execution::fill::Fees;

    #[derive(Default)]
    struct MockRepository {
        set_position: Option<fn(portfolio_id: &Uuid, position: &Position) -> Result<(), RepositoryError>>,
        get_position: Option<fn(position_id: &String) -> Result<Option<Position>, RepositoryError>>,
        remove_position: Option<fn(position_id: &String) -> Result<Option<Position>, RepositoryError>>,
        set_closed_position: Option<fn(portfolio_id: &Uuid, position: &Position) -> Result<(), RepositoryError>>,
        set_current_value: Option<fn(portfolio_id: &Uuid, value: f64)  -> Result<(), RepositoryError>>,
        get_current_value: Option<fn(portfolio_id: &Uuid) -> Result<f64, RepositoryError>>,
        set_current_cash: Option<fn(portfolio_id: &Uuid, cash: f64)  -> Result<(), RepositoryError>>,
        get_current_cash: Option<fn(portfolio_id: &Uuid) -> Result<f64, RepositoryError>>,
        position: Option<PositionBuilder>,
        value: Option<f64>,
        cash: Option<f64>,
    }

    impl PositionHandler for MockRepository {
        fn set_position(&mut self, portfolio_id: &Uuid, position: &Position) -> Result<(), RepositoryError> {
            self.position = Some(Position::builder()
                .direction(position.direction.clone())
                .current_symbol_price(position.current_symbol_price)
                .current_value_gross(position.current_value_gross)
                .enter_fees_total(position.enter_fees_total)
                .enter_value_gross(position.enter_value_gross)
                .enter_avg_price_gross(position.enter_avg_price_gross)
                .exit_fees_total(position.exit_fees_total)
                .exit_value_gross(position.exit_value_gross)
                .exit_avg_price_gross(position.exit_avg_price_gross)
                .unreal_profit_loss(position.unreal_profit_loss)
                .result_profit_loss(position.result_profit_loss));
            self.set_position.unwrap()(portfolio_id, position)
        }

        fn get_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError> {
            self.get_position.unwrap()(position_id)
        }

        fn remove_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError> {
            self.remove_position.unwrap()(position_id)
        }

        fn set_closed_position(&mut self, portfolio_id: &Uuid, position: &Position) -> Result<(), RepositoryError> {
            self.set_closed_position.unwrap()(portfolio_id, position)
        }
    }

    impl ValueHandler for MockRepository {
        fn set_current_value(&mut self, portfolio_id: &Uuid, value: f64) -> Result<(), RepositoryError> {
            self.value = Some(value);
            self.set_current_value.unwrap()(portfolio_id, value)
        }

        fn get_current_value(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError> {
            self.get_current_value.unwrap()(portfolio_id)
        }
    }

    impl CashHandler for MockRepository {
        fn set_current_cash(&mut self, portfolio_id: &Uuid, cash: f64) -> Result<(), RepositoryError> {
            self.cash = Some(cash);
            self.set_current_cash.unwrap()(portfolio_id, cash)
        }

        fn get_current_cash(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError> {
            self.get_current_cash.unwrap()(portfolio_id)
        }
    }

    fn build_mocked_portfolio<T>(mock_repository: T) -> Result<MetaPortfolio<T>, PortfolioError>
        where T: PositionHandler + ValueHandler + CashHandler {
        MetaPortfolio::builder()
            .id(Uuid::new_v4())
            .starting_cash(1000.0)
            .repository(mock_repository)
            .allocation_manager(DefaultAllocator{ default_order_value: 100.0 })
            .risk_manager(DefaultRisk{})
            .build()
    }

    #[test]
    fn update_from_market_with_long_position_increasing_in_value() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_position = Some(|_| {Ok(
            Some({
                let mut input_position = Position::default();
                input_position.direction = Direction::Long;
                input_position.quantity = 1.0;
                input_position.enter_fees_total = 3.0;
                input_position.current_symbol_price = 100.0;
                input_position.current_value_gross = 100.0;
                input_position.unreal_profit_loss = -3.0; // -3.0 from entry fees
                input_position
            })
        )});
        mock_repository.set_position = Some(|_, _| Ok(()));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input MarketEvent
        let mut input_market = MarketEvent::default();
        input_market.bar.close = 200.0; // +100.0 on input_position.current_symbol_price

        let result = portfolio.update_from_market(&input_market);
        let updated_position = portfolio.repository.position.unwrap();

        assert!(result.is_ok());
        assert_eq!(updated_position.current_symbol_price.unwrap(), 200.0);
        assert_eq!(updated_position.current_value_gross.unwrap(), 200.0);
        // Unreal PnL Long = current_value_gross - enter_value_gross - enter_fees_total*2
        assert_eq!(updated_position.unreal_profit_loss.unwrap(), 200.0 - 100.0 - 6.0);
    }

    #[test]
    fn update_from_market_with_long_position_decreasing_in_value() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_position = Some(|_| {Ok(
            Some({
                let mut input_position = Position::default();
                input_position.direction = Direction::Long;
                input_position.quantity = 1.0;
                input_position.enter_fees_total = 3.0;
                input_position.current_symbol_price = 100.0;
                input_position.current_value_gross = 100.0;
                input_position.unreal_profit_loss = -3.0; // -3.0 from entry fees
                input_position
            })
        )});
        mock_repository.set_position = Some(|_, _| Ok(()));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input MarketEvent
        let mut input_market = MarketEvent::default();
        input_market.bar.close = 50.0; // -50.0 on input_position.current_symbol_price

        let result = portfolio.update_from_market(&input_market);
        let updated_position = portfolio.repository.position.unwrap();

        assert!(result.is_ok());
        assert_eq!(updated_position.current_symbol_price.unwrap(), 50.0);
        assert_eq!(updated_position.current_value_gross.unwrap(), 50.0);
        // Unreal PnL Long = current_value_gross - enter_value_gross - enter_fees_total*2
        assert_eq!(updated_position.unreal_profit_loss.unwrap(), 50.0 - 100.0 - 6.0);
    }

    #[test]
    fn update_from_market_with_short_position_increasing_in_value() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_position = Some(|_| {Ok(
            Some({
                let mut input_position = Position::default();
                input_position.direction = Direction::Short;
                input_position.quantity = -1.0;
                input_position.enter_fees_total = 3.0;
                input_position.current_symbol_price = 100.0;
                input_position.current_value_gross = 100.0;
                input_position.unreal_profit_loss = -3.0; // -3.0 from entry fees
                input_position
            })
        )});
        mock_repository.set_position = Some(|_, _| Ok(()));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input MarketEvent
        let mut input_market = MarketEvent::default();
        input_market.bar.close = 50.0; // -50.0 on input_position.current_symbol_price

        let result = portfolio.update_from_market(&input_market);
        let updated_position = portfolio.repository.position.unwrap();

        assert!(result.is_ok());
        assert_eq!(updated_position.current_symbol_price.unwrap(), 50.0);
        assert_eq!(updated_position.current_value_gross.unwrap(), 50.0);
        // Unreal PnL Short = enter_value_gross - current_value_gross - enter_fees_total*2
        assert_eq!(updated_position.unreal_profit_loss.unwrap(), 100.0 - 50.0 - 6.0);
    }

    #[test]
    fn update_from_market_with_short_position_decreasing_in_value() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_position = Some(|_| {Ok(
            Some({
                let mut input_position = Position::default();
                input_position.direction = Direction::Short;
                input_position.quantity = -1.0;
                input_position.enter_fees_total = 3.0;
                input_position.current_symbol_price = 100.0;
                input_position.current_value_gross = 100.0;
                input_position.unreal_profit_loss = -3.0; // -3.0 from entry fees
                input_position
            })
        )});
        mock_repository.set_position = Some(|_, _| Ok(()));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input MarketEvent
        let mut input_market = MarketEvent::default();
        input_market.bar.close = 200.0; // +100.0 on input_position.current_symbol_price

        let result = portfolio.update_from_market(&input_market);
        let updated_position = portfolio.repository.position.unwrap();

        assert!(result.is_ok());
        assert_eq!(updated_position.current_symbol_price.unwrap(), 200.0);
        assert_eq!(updated_position.current_value_gross.unwrap(), 200.0);
        // Unreal PnL Short = enter_value_gross - current_value_gross - enter_fees_total*2
        assert_eq!(updated_position.unreal_profit_loss.unwrap(), 100.0 - 200.0 - 6.0);
    }

    #[test]
    fn generate_no_order_with_no_position_and_no_cash() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_position = Some(|_| Ok(None));
        mock_repository.get_current_cash = Some(|_| Ok(0.0));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input SignalEvent
        let input_signal = SignalEvent::default();

        let actual = portfolio.generate_order(&input_signal).unwrap();

        assert!(actual.is_none())
    }

    #[test]
    fn generate_no_order_with_position_and_no_cash() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_position = Some(|_| Ok(Some(Position::default())));
        mock_repository.get_current_cash = Some(|_| Ok(0.0));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input SignalEvent
        let input_signal = SignalEvent::default();

        let actual = portfolio.generate_order(&input_signal).unwrap();

        assert!(actual.is_none())
    }

    #[test]
    fn generate_order_long_with_no_position_and_input_net_long_signal() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_position = Some(|_| Ok(None));
        mock_repository.get_current_cash = Some(|_| Ok(100.0));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input SignalEvent
        let mut input_signal = SignalEvent::default();
        input_signal.signals.insert(Decision::Long, 1.0);

        let actual = portfolio.generate_order(&input_signal).unwrap().unwrap();

        assert_eq!(actual.decision, Decision::Long)
    }

    #[test]
    fn generate_order_short_with_no_position_and_input_net_short_signal() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_position = Some(|_| Ok(None));
        mock_repository.get_current_cash = Some(|_| Ok(100.0));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input SignalEvent
        let mut input_signal = SignalEvent::default();
        input_signal.signals.insert(Decision::Short, 1.0);

        let actual = portfolio.generate_order(&input_signal).unwrap().unwrap();

        assert_eq!(actual.decision, Decision::Short)
    }

    #[test]
    fn generate_order_close_long_with_long_position_and_input_net_close_long_signal() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_position = Some(|_| {Ok(
            Some({
                let mut position = Position::default();
                position.direction = Direction::Long;
                position
            })
        )});
        mock_repository.get_current_cash = Some(|_| Ok(100.0));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input SignalEvent
        let mut input_signal = SignalEvent::default();
        input_signal.signals.insert(Decision::CloseLong, 1.0);

        let actual = portfolio.generate_order(&input_signal).unwrap().unwrap();

        assert_eq!(actual.decision, Decision::CloseLong)
    }

    #[test]
    fn generate_order_close_short_with_short_position_and_input_net_close_short_signal() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_position = Some(|_| {Ok(
            Some({
                let mut position = Position::default();
                position.direction = Direction::Short;
                position
            })
        )});
        mock_repository.get_current_cash = Some(|_| Ok(100.0));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input SignalEvent
        let mut input_signal = SignalEvent::default();
        input_signal.signals.insert(Decision::CloseShort, 1.0);

        let actual = portfolio.generate_order(&input_signal).unwrap().unwrap();

        assert_eq!(actual.decision, Decision::CloseShort)
    }

    #[test]
    fn update_from_fill_entering_long_position() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_current_cash = Some(|_| Ok(200.0));
        mock_repository.get_current_value = Some(|_| Ok(200.0));
        mock_repository.remove_position = Some(|_| Ok(None));
        mock_repository.set_position = Some(|_, _| Ok(()));
        mock_repository.set_current_value = Some(|_, _| Ok(()));
        mock_repository.set_current_cash = Some(|_, _| Ok(()));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input FillEvent
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::Long;
        input_fill.quantity = 1.0;
        input_fill.fill_value_gross = 100.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        let result = portfolio.update_from_fill(&input_fill);
        let updated_repository = portfolio.repository;
        let entered_position = updated_repository.position.unwrap();
        let updated_cash = updated_repository.cash.unwrap();

        assert!(result.is_ok());
        assert_eq!(entered_position.direction.unwrap(), Direction::Long);
        assert_eq!(entered_position.enter_value_gross.unwrap(), 100.0);
        assert_eq!(entered_position.enter_fees_total.unwrap(), 3.0);
        assert_eq!(updated_cash, 200.0 - 100.0 - 3.0); // cash += enter_value_gross - enter_fees
    }

    #[test]
    fn update_from_fill_entering_short_position() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_current_cash = Some(|_| Ok(200.0));
        mock_repository.get_current_value = Some(|_| Ok(200.0));
        mock_repository.remove_position = Some(|_| Ok(None));
        mock_repository.set_position = Some(|_, _| Ok(()));
        mock_repository.set_current_value = Some(|_, _| Ok(()));
        mock_repository.set_current_cash = Some(|_, _| Ok(()));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input FillEvent
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::Short;
        input_fill.quantity = -1.0;
        input_fill.fill_value_gross = 100.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        let result = portfolio.update_from_fill(&input_fill);
        let updated_repository = portfolio.repository;
        let entered_position = updated_repository.position.unwrap();
        let updated_cash = updated_repository.cash.unwrap();

        assert!(result.is_ok());
        assert_eq!(entered_position.direction.unwrap(), Direction::Short);
        assert_eq!(entered_position.enter_value_gross.unwrap(), 100.0);
        assert_eq!(entered_position.enter_fees_total.unwrap(), 3.0);
        assert_eq!(updated_cash, 200.0 - 100.0 - 3.0); // cash += enter_value_gross - enter_fees
    }

    #[test]
    fn update_from_fill_exiting_long_position_in_profit() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_current_cash = Some(|_| Ok(97.0));
        mock_repository.get_current_value = Some(|_| Ok(200.0));
        mock_repository.remove_position = Some(|_| {Ok({
            Some({
                let mut input_position = Position::default();
                input_position.direction = Direction::Long;
                input_position.quantity = 1.0;
                input_position.enter_fees_total = 3.0;
                input_position.enter_value_gross = 100.0;
                input_position
            })
        })});
        mock_repository.set_closed_position = Some(|_, _| Ok(()));
        mock_repository.set_current_value = Some(|_, _| Ok(()));
        mock_repository.set_current_cash = Some(|_, _| Ok(()));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input FillEvent
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::CloseLong;
        input_fill.quantity = -1.0;
        input_fill.fill_value_gross = 200.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        let result = portfolio.update_from_fill(&input_fill);
        let updated_repository = portfolio.repository;
        let updated_cash = updated_repository.cash.unwrap();
        let updated_value = updated_repository.value.unwrap();

        assert!(result.is_ok());
        // LONG result_profit_loss = exit_value_gross - enter_value_gross - total_fees
        // cash += enter_value_gross + result_profit_loss + enter_fees_total
        assert_eq!(updated_cash, 97.0 + 100.0 + (200.0 - 100.0 - 6.0) + 3.0);
        // value += result_profit_loss
        assert_eq!(updated_value, 200.0 + (200.0 - 100.0 - 6.0));
    }

    #[test]
    fn update_from_fill_exiting_long_position_in_loss() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_current_cash = Some(|_| Ok(97.0));
        mock_repository.get_current_value = Some(|_| Ok(200.0));
        mock_repository.remove_position = Some(|_| {Ok({
            Some({
                let mut input_position = Position::default();
                input_position.direction = Direction::Long;
                input_position.quantity = 1.0;
                input_position.enter_fees_total = 3.0;
                input_position.enter_value_gross = 100.0;
                input_position
            })
        })});
        mock_repository.set_closed_position = Some(|_, _| Ok(()));
        mock_repository.set_current_value = Some(|_, _| Ok(()));
        mock_repository.set_current_cash = Some(|_, _| Ok(()));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input FillEvent
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::CloseLong;
        input_fill.quantity = -1.0;
        input_fill.fill_value_gross = 50.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        let result = portfolio.update_from_fill(&input_fill);
        let updated_repository = portfolio.repository;
        let updated_cash = updated_repository.cash.unwrap();
        let updated_value = updated_repository.value.unwrap();

        assert!(result.is_ok());
        // LONG result_profit_loss = exit_value_gross - enter_value_gross - total_fees
        // cash += enter_value_gross + result_profit_loss + enter_fees_total
        assert_eq!(updated_cash, 97.0 + 100.0 + (50.0 - 100.0 - 6.0) + 3.0);
        // value += result_profit_loss
        assert_eq!(updated_value, 200.0 + (50.0 - 100.0 - 6.0));
    }

    #[test]
    fn update_from_fill_exiting_short_position_in_profit() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_current_cash = Some(|_| Ok(97.0));
        mock_repository.get_current_value = Some(|_| Ok(200.0));
        mock_repository.remove_position = Some(|_| {Ok({
            Some({
                let mut input_position = Position::default();
                input_position.direction = Direction::Short;
                input_position.quantity = -1.0;
                input_position.enter_fees_total = 3.0;
                input_position.enter_value_gross = 100.0;
                input_position
            })
        })});
        mock_repository.set_closed_position = Some(|_, _| Ok(()));
        mock_repository.set_current_value = Some(|_, _| Ok(()));
        mock_repository.set_current_cash = Some(|_, _| Ok(()));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input FillEvent
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::CloseShort;
        input_fill.quantity = 1.0;
        input_fill.fill_value_gross = 50.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        let result = portfolio.update_from_fill(&input_fill);
        let updated_repository = portfolio.repository;
        let updated_cash = updated_repository.cash.unwrap();
        let updated_value = updated_repository.value.unwrap();

        assert!(result.is_ok());
        // SHORT result_profit_loss = enter_value_gross - exit_value_gross - total_fees
        // cash += enter_value_gross + result_profit_loss + enter_fees_total
        assert_eq!(updated_cash, 97.0 + 100.0 + (100.0 - 50.0 - 6.0) + 3.0);
        // value += result_profit_loss
        assert_eq!(updated_value, 200.0 + (100.0 - 50.0 - 6.0));
    }

    #[test]
    fn update_from_fill_exiting_short_position_in_loss() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_current_cash = Some(|_| Ok(97.0));
        mock_repository.get_current_value = Some(|_| Ok(200.0));
        mock_repository.remove_position = Some(|_| {Ok({
            Some({
                let mut input_position = Position::default();
                input_position.direction = Direction::Short;
                input_position.quantity = -1.0;
                input_position.enter_fees_total = 3.0;
                input_position.enter_value_gross = 100.0;
                input_position
            })
        })});
        mock_repository.set_closed_position = Some(|_, _| Ok(()));
        mock_repository.set_current_value = Some(|_, _| Ok(()));
        mock_repository.set_current_cash = Some(|_, _| Ok(()));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input FillEvent
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::CloseShort;
        input_fill.quantity = 1.0;
        input_fill.fill_value_gross = 150.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        let result = portfolio.update_from_fill(&input_fill);
        let updated_repository = portfolio.repository;
        let updated_cash = updated_repository.cash.unwrap();
        let updated_value = updated_repository.value.unwrap();

        assert!(result.is_ok());
        // SHORT result_profit_loss = enter_value_gross - exit_value_gross - total_fees
        // cash += enter_value_gross + result_profit_loss + enter_fees_total
        assert_eq!(updated_cash, 97.0 + 100.0 + (100.0 - 150.0 - 6.0) + 3.0);
        // value += result_profit_loss
        assert_eq!(updated_value, 200.0 + (100.0 - 150.0 - 6.0));
    }

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
    fn parse_signal_decisions_to_none_with_some_long_position_and_long_signal() {
        // Some(Position)
        let mut position = Position::default();
        position.direction = Direction::Long;
        let position = Some(position);
        let position = position.as_ref();

        // Signals HashMap
        let mut signals = HashMap::with_capacity(4);
        signals.insert(Decision::Long, 1.0);
        signals.insert(Decision::CloseShort, 1.0);

        let actual = parse_signal_decisions(&position, &signals);

        assert!(actual.is_none())
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
    fn parse_signal_decisions_to_none_with_some_short_position_and_short_signal() {
        // Some(Position)
        let mut position = Position::default();
        position.direction = Direction::Short;
        let position = Some(position);
        let position = position.as_ref();

        // Signals HashMap
        let mut signals = HashMap::with_capacity(4);
        signals.insert(Decision::CloseLong, 1.0);
        signals.insert(Decision::Short, 1.0);

        let actual = parse_signal_decisions(&position, &signals);

        assert!(actual.is_none())
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