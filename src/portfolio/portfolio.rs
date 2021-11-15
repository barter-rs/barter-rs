use crate::data::market::MarketEvent;
use crate::execution::fill::FillEvent;
use crate::portfolio::allocator::{DefaultAllocator, OrderAllocator};
use crate::portfolio::error::PortfolioError;
use crate::portfolio::order::{OrderEvent, OrderType};
use crate::portfolio::position::{
    Direction, Position, PositionEnterer, PositionExiter, PositionUpdater,
};
use crate::portfolio::repository::{
    determine_position_id, CashHandler, PositionHandler, ValueHandler,
};
use crate::portfolio::risk::{DefaultRisk, OrderEvaluator};
use crate::portfolio::{FillUpdater, MarketUpdater, OrderGenerator};
use crate::strategy::signal::{Decision, SignalEvent, SignalStrength};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;
use crate::portfolio::repository::error::RepositoryError;

/// Lego components for constructing & initialising a [MetaPortfolio] via the init() constructor
/// method.
#[derive(Debug)]
pub struct PortfolioLego {
    pub allocator: DefaultAllocator,
    pub risk: DefaultRisk,
    pub starting_cash: f64,
}

/// Portfolio with state persisted in a repository. Implements [MarketUpdater], [OrderGenerator],
/// and [FillUpdater]. The Portfolio analyses an advisory [SignalEvent] from a Strategy and decides
/// whether to place a corresponding [OrderEvent]. If a [Position] is opened, the Portfolio keeps
/// track the it's state, as well as it's own.
pub struct MetaPortfolio<T>
where
    T: PositionHandler + ValueHandler + CashHandler,
{
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

impl<T> MarketUpdater for MetaPortfolio<T>
where
    T: PositionHandler + ValueHandler + CashHandler,
{
    fn update_from_market(&mut self, market: &MarketEvent) -> Result<(), PortfolioError> {
        // Determine the position_id & associated Option<Position> related to the input MarketEvent
        let position_id = determine_position_id(&self.id, &market.exchange, &market.symbol);

        // If Portfolio contains an open Position for the MarketEvent Symbol-Exchange combination
        if let Some(mut position) = self.repository.get_position(&position_id)? {
            // Update Position
            position.update(market);
            self.repository.set_position(&self.id, position)?;
        }

        Ok(())
    }
}

impl<T> OrderGenerator for MetaPortfolio<T>
where
    T: PositionHandler + ValueHandler + CashHandler,
{
    fn generate_order(
        &mut self,
        signal: &SignalEvent,
    ) -> Result<Option<OrderEvent>, PortfolioError> {
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
            market_meta: signal.market_meta.clone(),
            decision: signal_decision.clone(),
            quantity: 0.0,
            order_type: OrderType::default(),
        };

        // OrderEvent size allocation
        order = self
            .allocation_manager
            .allocate_order(order, position, *signal_strength)?;

        // OrderEvent risk evaluation - refine or cancel
        Ok(self.risk_manager.evaluate_order(order)?)
    }
}

impl<T> FillUpdater for MetaPortfolio<T>
where
    T: PositionHandler + ValueHandler + CashHandler,
{
    fn update_from_fill(&mut self, fill: &FillEvent) -> Result<(), PortfolioError> {
        // Get the Portfolio Cash from Repository
        let mut current_cash = self.repository.get_current_cash(&self.id)?;

        // Determine the position_id that is related to the input FillEvent
        let position_id = determine_position_id(&self.id, &fill.exchange, &fill.symbol);

        // EXIT SCENARIO - FillEvent for Symbol-Exchange with open Position
        if let Some(mut position) = self.repository.remove_position(&position_id)? {
            // Get the Portfolio Value from the Repository
            let mut current_value = self.repository.get_current_value(&self.id)?;

            // Exit Position & persist in Repository closed_positions
            position.exit(current_value, fill)?;

            // Update Portfolio cash on exit - enter_total_fees added since included in result PnL calc
            current_cash += position.enter_value_gross
                + position.result_profit_loss
                + position.enter_fees_total;

            // Update Portfolio value after exit & persist in Repository
            current_value += position.result_profit_loss;

            // Persist updated Portfolio value & exited Position in Repository
            self.repository.set_closed_position(&self.id, position)?;
            self.repository.set_current_value(&self.id, current_value)?;
        }
        // ENTRY SCENARIO - FillEvent for Symbol-Exchange with no Position
        else {
            let position = Position::enter(&fill)?;

            // Update Portfolio cash entry
            current_cash += -position.enter_value_gross - position.enter_fees_total;

            // Add to current Positions in Repository
            self.repository.set_position(&self.id, position)?;
        }

        // Persist updated Portfolio cash in Repository
        self.repository.set_current_cash(&self.id, current_cash)?;

        Ok(())
    }
}

impl<T> PositionHandler for MetaPortfolio<T>
where
    T: PositionHandler + ValueHandler + CashHandler,
{
    fn set_position(&mut self, _: &Uuid, position: Position) -> Result<(), RepositoryError> {
        self.repository.set_position(&self.id, position)
    }

    fn get_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError> {
        self.repository.get_position(position_id)
    }

    fn remove_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError> {
        self.repository.remove_position(position_id)
    }

    fn set_closed_position(&mut self, _: &Uuid, position: Position) -> Result<(), RepositoryError> {
        self.repository.set_closed_position(&self.id, position)
    }

    fn get_closed_positions(&mut self, _: &Uuid) -> Result<Option<Vec<Position>>, RepositoryError> {
        self.repository.get_closed_positions(&self.id)
    }
}

impl<T> MetaPortfolio<T>
where
    T: PositionHandler + ValueHandler + CashHandler,
{
    /// Constructs a new [MetaPortfolio] component using the provided [PortfolioLego] components, and
    /// persists the initial [MetaPortfolio] state in the Repository.
    pub fn init(lego: PortfolioLego, repository: T) -> Result<Self, PortfolioError> {
        // Construct MetaPortfolio instance
        let mut portfolio = Self {
            id: Uuid::new_v4(),
            starting_cash: lego.starting_cash,
            repository,
            allocation_manager: lego.allocator,
            risk_manager: lego.risk,
        };

        // Initialise MetaPortfolio state in the Repository
        portfolio.repository.set_current_cash(&portfolio.id, portfolio.starting_cash)?;
        portfolio.repository.set_current_value(&portfolio.id, portfolio.starting_cash)?;

        Ok(portfolio)
    }

    /// Returns a [MetaPortfolioBuilder] instance.
    pub fn builder() -> MetaPortfolioBuilder<T> {
        MetaPortfolioBuilder::new()
    }

    /// Determines if the Portfolio has any cash to enter a new [Position].
    fn no_cash_to_enter_new_position(
        &mut self,
        position: &Option<&Position>,
    ) -> Result<bool, PortfolioError> {
        let current_cash = self.repository.get_current_cash(&self.id)?;
        Ok(position.is_none() && current_cash == 0.0)
    }
}

/// Builder to construct [MetaPortfolio] instances.
#[derive(Debug, Default)]
pub struct MetaPortfolioBuilder<T>
where
    T: PositionHandler + ValueHandler + CashHandler,
{
    id: Option<Uuid>,
    starting_cash: Option<f64>,
    repository: Option<T>,
    allocation_manager: Option<DefaultAllocator>,
    risk_manager: Option<DefaultRisk>,
}

impl<T> MetaPortfolioBuilder<T>
where
    T: PositionHandler + ValueHandler + CashHandler,
{
    pub fn new() -> Self {
        Self {
            id: None,
            starting_cash: None,
            repository: None,
            allocation_manager: None,
            risk_manager: None,
        }
    }

    pub fn id(self, value: Uuid) -> Self {
        Self {
            id: Some(value),
            ..self
        }
    }

    pub fn starting_cash(self, value: f64) -> Self {
        Self {
            starting_cash: Some(value),
            ..self
        }
    }

    pub fn repository(self, value: T) -> Self {
        Self {
            repository: Some(value),
            ..self
        }
    }

    pub fn allocation_manager(self, value: DefaultAllocator) -> Self {
        Self {
            allocation_manager: Some(value),
            ..self
        }
    }

    pub fn risk_manager(self, value: DefaultRisk) -> Self {
        Self {
            risk_manager: Some(value),
            ..self
        }
    }

    pub fn build_and_init(self) -> Result<MetaPortfolio<T>, PortfolioError> {
        let id = self.id.ok_or(PortfolioError::BuilderIncomplete)?;
        let starting_cash = self
            .starting_cash
            .ok_or(PortfolioError::BuilderIncomplete)?;
        let repository = self.repository.ok_or(PortfolioError::BuilderIncomplete)?;
        let allocation_manager = self
            .allocation_manager
            .ok_or(PortfolioError::BuilderIncomplete)?;
        let risk_manager = self.risk_manager.ok_or(PortfolioError::BuilderIncomplete)?;

        let mut portfolio = MetaPortfolio {
            id,
            starting_cash,
            repository,
            allocation_manager,
            risk_manager,
        };

        // Initialise MetaPortfolio state in the Repository
        portfolio.repository.set_current_cash(&id, starting_cash)?;
        portfolio.repository.set_current_value(&id, starting_cash)?;

        Ok(portfolio)
    }
}

/// Parses an incoming [SignalEvent]'s signals map. Determines what the net signal [Decision] will
/// be, and it's associated [SignalStrength].
pub fn parse_signal_decisions<'a>(
    position: &'a Option<&Position>,
    signals: &'a HashMap<Decision, SignalStrength>,
) -> Option<(&'a Decision, &'a SignalStrength)> {
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
        };
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
    use crate::execution::fill::Fees;
    use crate::portfolio::position::PositionBuilder;
    use crate::portfolio::repository::error::RepositoryError;
    use barter_data::model::MarketData;

    #[derive(Default)]
    struct MockRepository {
        set_position:
            Option<fn(portfolio_id: &Uuid, position: Position) -> Result<(), RepositoryError>>,
        get_position: Option<fn(position_id: &String) -> Result<Option<Position>, RepositoryError>>,
        remove_position:
            Option<fn(position_id: &String) -> Result<Option<Position>, RepositoryError>>,
        set_closed_position:
            Option<fn(portfolio_id: &Uuid, position: Position) -> Result<(), RepositoryError>>,
        get_closed_positions:
            Option<fn(portfolio_id: &Uuid) -> Result<Option<Vec<Position>>, RepositoryError>>,
        set_current_value:
            Option<fn(portfolio_id: &Uuid, value: f64) -> Result<(), RepositoryError>>,
        get_current_value: Option<fn(portfolio_id: &Uuid) -> Result<f64, RepositoryError>>,
        set_current_cash: Option<fn(portfolio_id: &Uuid, cash: f64) -> Result<(), RepositoryError>>,
        get_current_cash: Option<fn(portfolio_id: &Uuid) -> Result<f64, RepositoryError>>,
        position: Option<PositionBuilder>,
        value: Option<f64>,
        cash: Option<f64>,
    }

    impl PositionHandler for MockRepository {
        fn set_position(
            &mut self,
            portfolio_id: &Uuid,
            position: Position,
        ) -> Result<(), RepositoryError> {
            self.position = Some(
                Position::builder()
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
                    .result_profit_loss(position.result_profit_loss),
            );
            self.set_position.unwrap()(portfolio_id, position)
        }

        fn get_position(
            &mut self,
            position_id: &String,
        ) -> Result<Option<Position>, RepositoryError> {
            self.get_position.unwrap()(position_id)
        }

        fn remove_position(
            &mut self,
            position_id: &String,
        ) -> Result<Option<Position>, RepositoryError> {
            self.remove_position.unwrap()(position_id)
        }

        fn set_closed_position(
            &mut self,
            portfolio_id: &Uuid,
            position: Position,
        ) -> Result<(), RepositoryError> {
            self.set_closed_position.unwrap()(portfolio_id, position)
        }

        fn get_closed_positions(
            &mut self,
            portfolio_id: &Uuid,
        ) -> Result<Option<Vec<Position>>, RepositoryError> {
            self.get_closed_positions.unwrap()(portfolio_id)
        }
    }

    impl ValueHandler for MockRepository {
        fn set_current_value(
            &mut self,
            portfolio_id: &Uuid,
            value: f64,
        ) -> Result<(), RepositoryError> {
            self.value = Some(value);
            self.set_current_value.unwrap()(portfolio_id, value)
        }

        fn get_current_value(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError> {
            self.get_current_value.unwrap()(portfolio_id)
        }
    }

    impl CashHandler for MockRepository {
        fn set_current_cash(
            &mut self,
            portfolio_id: &Uuid,
            cash: f64,
        ) -> Result<(), RepositoryError> {
            self.cash = Some(cash);
            self.set_current_cash.unwrap()(portfolio_id, cash)
        }

        fn get_current_cash(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError> {
            self.get_current_cash.unwrap()(portfolio_id)
        }
    }

    fn build_mocked_portfolio<T>(mock_repository: T) -> Result<MetaPortfolio<T>, PortfolioError>
    where
        T: PositionHandler + ValueHandler + CashHandler,
    {
        let builder = MetaPortfolio::builder()
            .id(Uuid::new_v4())
            .starting_cash(1000.0)
            .repository(mock_repository)
            .allocation_manager(DefaultAllocator {
                default_order_value: 100.0,
            })
            .risk_manager(DefaultRisk {});

        build_uninitialised_portfolio(builder)
    }

    fn build_uninitialised_portfolio<T>(builder: MetaPortfolioBuilder<T>) -> Result<MetaPortfolio<T>, PortfolioError>
    where
        T: PositionHandler + ValueHandler + CashHandler,
    {
        let id = builder.id.ok_or(PortfolioError::BuilderIncomplete)?;
        let starting_cash = builder.starting_cash.ok_or(PortfolioError::BuilderIncomplete)?;
        let repository = builder.repository.ok_or(PortfolioError::BuilderIncomplete)?;
        let allocation_manager = builder.allocation_manager.ok_or(PortfolioError::BuilderIncomplete)?;
        let risk_manager = builder.risk_manager.ok_or(PortfolioError::BuilderIncomplete)?;

        Ok(MetaPortfolio {
            id,
            starting_cash,
            repository,
            allocation_manager,
            risk_manager,
        })
    }

    #[test]
    fn update_from_market_with_long_position_increasing_in_value() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_position = Some(|_| {
            Ok(Some({
                let mut input_position = Position::default();
                input_position.direction = Direction::Long;
                input_position.quantity = 1.0;
                input_position.enter_fees_total = 3.0;
                input_position.current_symbol_price = 100.0;
                input_position.current_value_gross = 100.0;
                input_position.unreal_profit_loss = -3.0; // -3.0 from entry fees
                input_position
            }))
        });
        mock_repository.set_position = Some(|_, _| Ok(()));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input MarketEvent
        let mut input_market = MarketEvent::default();
        match input_market.data {
            // candle.close +100.0 on input_position.current_symbol_price
            MarketData::Candle(ref mut candle) => candle.close = 200.0,
            MarketData::Trade(ref mut trade) => trade.price = 200.0,
        };

        let result = portfolio.update_from_market(&input_market);
        let updated_position = portfolio.repository.position.unwrap();

        assert!(result.is_ok());
        assert_eq!(updated_position.current_symbol_price.unwrap(), 200.0);
        assert_eq!(updated_position.current_value_gross.unwrap(), 200.0);
        // Unreal PnL Long = current_value_gross - enter_value_gross - enter_fees_total*2
        assert_eq!(
            updated_position.unreal_profit_loss.unwrap(),
            200.0 - 100.0 - 6.0
        );
    }

    #[test]
    fn update_from_market_with_long_position_decreasing_in_value() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_position = Some(|_| {
            Ok(Some({
                let mut input_position = Position::default();
                input_position.direction = Direction::Long;
                input_position.quantity = 1.0;
                input_position.enter_fees_total = 3.0;
                input_position.current_symbol_price = 100.0;
                input_position.current_value_gross = 100.0;
                input_position.unreal_profit_loss = -3.0; // -3.0 from entry fees
                input_position
            }))
        });
        mock_repository.set_position = Some(|_, _| Ok(()));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input MarketEvent
        let mut input_market = MarketEvent::default();
        match input_market.data {
            // -50.0 on input_position.current_symbol_price
            MarketData::Candle(ref mut candle) => candle.close = 50.0,
            MarketData::Trade(ref mut trade) => trade.price = 50.0,
        };

        let result = portfolio.update_from_market(&input_market);
        let updated_position = portfolio.repository.position.unwrap();

        assert!(result.is_ok());
        assert_eq!(updated_position.current_symbol_price.unwrap(), 50.0);
        assert_eq!(updated_position.current_value_gross.unwrap(), 50.0);
        // Unreal PnL Long = current_value_gross - enter_value_gross - enter_fees_total*2
        assert_eq!(
            updated_position.unreal_profit_loss.unwrap(),
            50.0 - 100.0 - 6.0
        );
    }

    #[test]
    fn update_from_market_with_short_position_increasing_in_value() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_position = Some(|_| {
            Ok(Some({
                let mut input_position = Position::default();
                input_position.direction = Direction::Short;
                input_position.quantity = -1.0;
                input_position.enter_fees_total = 3.0;
                input_position.current_symbol_price = 100.0;
                input_position.current_value_gross = 100.0;
                input_position.unreal_profit_loss = -3.0; // -3.0 from entry fees
                input_position
            }))
        });
        mock_repository.set_position = Some(|_, _| Ok(()));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input MarketEvent
        let mut input_market = MarketEvent::default();
        match input_market.data {
            // -50.0 on input_position.current_symbol_price
            MarketData::Candle(ref mut candle) => candle.close = 50.0,
            MarketData::Trade(ref mut trade) => trade.price = 50.0,
        };

        let result = portfolio.update_from_market(&input_market);
        let updated_position = portfolio.repository.position.unwrap();

        assert!(result.is_ok());
        assert_eq!(updated_position.current_symbol_price.unwrap(), 50.0);
        assert_eq!(updated_position.current_value_gross.unwrap(), 50.0);
        // Unreal PnL Short = enter_value_gross - current_value_gross - enter_fees_total*2
        assert_eq!(
            updated_position.unreal_profit_loss.unwrap(),
            100.0 - 50.0 - 6.0
        );
    }

    #[test]
    fn update_from_market_with_short_position_decreasing_in_value() {
        // Build Portfolio
        let mut mock_repository = MockRepository::default();
        mock_repository.get_position = Some(|_| {
            Ok(Some({
                let mut input_position = Position::default();
                input_position.direction = Direction::Short;
                input_position.quantity = -1.0;
                input_position.enter_fees_total = 3.0;
                input_position.current_symbol_price = 100.0;
                input_position.current_value_gross = 100.0;
                input_position.unreal_profit_loss = -3.0; // -3.0 from entry fees
                input_position
            }))
        });
        mock_repository.set_position = Some(|_, _| Ok(()));
        let mut portfolio = build_mocked_portfolio(mock_repository).unwrap();

        // Input MarketEvent
        let mut input_market = MarketEvent::default();

        match input_market.data {
            // +100.0 on input_position.current_symbol_price
            MarketData::Candle(ref mut candle) => candle.close = 200.0,
            MarketData::Trade(ref mut trade) => trade.price = 200.0,
        };

        let result = portfolio.update_from_market(&input_market);
        let updated_position = portfolio.repository.position.unwrap();

        assert!(result.is_ok());
        assert_eq!(updated_position.current_symbol_price.unwrap(), 200.0);
        assert_eq!(updated_position.current_value_gross.unwrap(), 200.0);
        // Unreal PnL Short = enter_value_gross - current_value_gross - enter_fees_total*2
        assert_eq!(
            updated_position.unreal_profit_loss.unwrap(),
            100.0 - 200.0 - 6.0
        );
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
        mock_repository.get_position = Some(|_| {
            Ok(Some({
                let mut position = Position::default();
                position.direction = Direction::Long;
                position
            }))
        });
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
        mock_repository.get_position = Some(|_| {
            Ok(Some({
                let mut position = Position::default();
                position.direction = Direction::Short;
                position
            }))
        });
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
            network: 1.0,
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
            network: 1.0,
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
        mock_repository.remove_position = Some(|_| {
            Ok({
                Some({
                    let mut input_position = Position::default();
                    input_position.direction = Direction::Long;
                    input_position.quantity = 1.0;
                    input_position.enter_fees_total = 3.0;
                    input_position.enter_value_gross = 100.0;
                    input_position
                })
            })
        });
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
            network: 1.0,
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
        mock_repository.remove_position = Some(|_| {
            Ok({
                Some({
                    let mut input_position = Position::default();
                    input_position.direction = Direction::Long;
                    input_position.quantity = 1.0;
                    input_position.enter_fees_total = 3.0;
                    input_position.enter_value_gross = 100.0;
                    input_position
                })
            })
        });
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
            network: 1.0,
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
        mock_repository.remove_position = Some(|_| {
            Ok({
                Some({
                    let mut input_position = Position::default();
                    input_position.direction = Direction::Short;
                    input_position.quantity = -1.0;
                    input_position.enter_fees_total = 3.0;
                    input_position.enter_value_gross = 100.0;
                    input_position
                })
            })
        });
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
            network: 1.0,
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
        mock_repository.remove_position = Some(|_| {
            Ok({
                Some({
                    let mut input_position = Position::default();
                    input_position.direction = Direction::Short;
                    input_position.quantity = -1.0;
                    input_position.enter_fees_total = 3.0;
                    input_position.enter_value_gross = 100.0;
                    input_position
                })
            })
        });
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
            network: 1.0,
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
