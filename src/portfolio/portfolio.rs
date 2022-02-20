use std::collections::HashMap;
use std::marker::PhantomData;

use chrono::Utc;
use serde::Serialize;
use tracing::info;
use uuid::Uuid;

use crate::{determine_market_id, Market, MarketId};
use crate::event::Event;
use crate::data::{MarketEvent, MarketMeta};
use crate::strategy::{Decision, SignalEvent, SignalForceExit, SignalStrength};
use crate::portfolio::{Balance, FillUpdater, MarketUpdater, OrderEvent, OrderGenerator, OrderType};
use crate::portfolio::allocator::OrderAllocator;
use crate::portfolio::error::PortfolioError;
use crate::portfolio::position::{
    determine_position_id, Direction, Position, PositionEnterer, PositionExiter, PositionId,
    PositionUpdate, PositionUpdater,
};
use crate::portfolio::repository::{BalanceHandler, error::RepositoryError, PositionHandler, StatisticHandler};
use crate::portfolio::risk::OrderEvaluator;
use crate::statistic::summary::{Initialiser, PositionSummariser};
use crate::execution::FillEvent;

/// Lego components for constructing & initialising a [`MetaPortfolio`] via the init() constructor
/// method.
#[derive(Debug)]
pub struct PortfolioLego<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    /// Identifier for the [`Engine`] a [`MetaPortfolio`] is associated with (1-to-1 relationship).
    pub engine_id: Uuid,
    /// [`Market`]s being tracked by a [`MetaPortfolio`].
    pub markets: Vec<Market>,
    /// Repository for a [`MetaPortfolio`] to persist it's state in. Implements
    /// [`PositionHandler`], [`EquityHandler`], [`CashHandler`], and [`StatisticHandler`]
    pub repository: Repository,
    /// Allocation manager implements [`OrderAllocator`].
    pub allocator: Allocator,
    /// Risk manager implements [`OrderEvaluator`].
    pub risk: RiskManager,
    /// Cash balance a [`MetaPortfolio`] starts with.
    pub starting_cash: f64,
    /// Configuration used to initialise the Statistics for every Market's performance tracked by a
    /// [`MetaPortfolio`].
    pub statistic_config: Statistic::Config,
    pub _statistic_marker: PhantomData<Statistic>,
}

/// Portfolio with state persisted in a repository. [`MarketUpdater`], [`OrderGenerator`],
/// [`FillUpdater`] and [`PositionHandler`].
#[derive(Debug)]
pub struct MetaPortfolio<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    /// Identifier for the [`Engine`] this Portfolio is associated with (1-to-1 relationship).
    engine_id: Uuid,
    /// Repository for the [`MetaPortfolio`] to persist it's state in. Implements
    /// [`PositionHandler`], [`EquityHandler`], [`CashHandler`], and [`StatisticHandler`]
    repository: Repository,
    /// Allocation manager implements [`OrderAllocator`].
    allocation_manager: Allocator,
    /// Risk manager implements [`OrderEvaluator`].
    risk_manager: RiskManager,
    _statistic_marker: PhantomData<Statistic>,
}

impl<Repository, Allocator, RiskManager, Statistic> MarketUpdater
    for MetaPortfolio<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    fn update_from_market(
        &mut self,
        market: &MarketEvent,
    ) -> Result<Option<PositionUpdate>, PortfolioError> {
        // Determine the position_id associated to the input MarketEvent
        let position_id = determine_position_id(self.engine_id, market.exchange, &market.symbol);

        // Update Position if Portfolio has an open Position for that Symbol-Exchange combination
        if let Some(mut position) = self.repository.get_open_position(&position_id)? {
            // Derive PositionUpdate event that communicates the open Position's change in state
            let position_update = position.update(market);

            // Save updated open Position in the repository
            self.repository.set_open_position(position)?;

            Ok(Some(position_update))
        } else {
            Ok(None)
        }
    }
}

impl<Repository, Allocator, RiskManager, Statistic> OrderGenerator
    for MetaPortfolio<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    fn generate_order(
        &mut self,
        signal: &SignalEvent,
    ) -> Result<Option<OrderEvent>, PortfolioError> {
        // Determine the position_id & associated Option<Position> related to input SignalEvent
        let position_id = determine_position_id(self.engine_id, signal.exchange, &signal.symbol);
        let position = self.repository.get_open_position(&position_id)?;

        // If signal is advising to open a new Position rather than close one, check we have cash
        if position.is_none() && self.no_cash_to_enter_new_position()? {
            return Ok(None);
        }

        // Parse signals from Strategy to determine net signal decision & associated strength
        let position = position.as_ref();
        let (signal_decision, signal_strength) =
            match parse_signal_decisions(&position, &signal.signals) {
                None => return Ok(None),
                Some(net_signal) => net_signal,
            };

        // Construct mutable OrderEvent that can be modified by Allocation & Risk management
        let mut order = OrderEvent {
            event_type: OrderEvent::ORGANIC_ORDER,
            trace_id: signal.trace_id,
            timestamp: Utc::now(),
            exchange: signal.exchange,
            symbol: signal.symbol.clone(),
            market_meta: signal.market_meta,
            decision: *signal_decision,
            quantity: 0.0,
            order_type: OrderType::default(),
        };

        // Manage OrderEvent size allocation
        self.allocation_manager
            .allocate_order(&mut order, position, *signal_strength);

        // Manage global risk when evaluating OrderEvent - keep the same, refine or cancel
        Ok(self.risk_manager.evaluate_order(order))
    }

    fn generate_exit_order(
        &mut self,
        signal: SignalForceExit,
    ) -> Result<Option<OrderEvent>, PortfolioError> {
        // Determine PositionId associated with the SignalForceExit
        let position_id = determine_position_id(self.engine_id, signal.exchange, &signal.symbol);

        // Retrieve Option<Position> associated with the PositionId
        let position = match self.repository.get_open_position(&position_id)? {
            None => {
                info!(
                    position_id = &*position_id,
                    outcome = "no forced exit OrderEvent generated",
                    "cannot generate forced exit OrderEvent for a Position that isn't open"
                );
                return Ok(None);
            }
            Some(position) => position,
        };

        Ok(Some(OrderEvent {
            event_type: OrderEvent::FORCED_EXIT_ORDER,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            exchange: &*signal.exchange,
            symbol: signal.symbol,
            market_meta: MarketMeta {
                close: position.current_symbol_price,
                timestamp: position.meta.last_update_timestamp,
            },
            decision: position.direction.determine_exit_decision(),
            quantity: 0.0 - position.quantity,
            order_type: OrderType::Market,
        }))
    }
}

impl<Repository, Allocator, RiskManager, Statistic> FillUpdater
    for MetaPortfolio<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser + Serialize,
{
    fn update_from_fill(
        &mut self,
        fill: &FillEvent,
    ) -> Result<Vec<Event>, PortfolioError> {
        // Allocate Vector<Event> to contain any update_from_fill generated events
        let mut generated_events: Vec<Event> = Vec::with_capacity(2);

        // Get the Portfolio Balance from Repository & update timestamp
        let mut balance = self.repository.get_balance(self.engine_id)?;
        balance.timestamp = fill.timestamp;

        // Determine the position_id that is related to the input FillEvent
        let position_id = determine_position_id(self.engine_id, fill.exchange, &fill.symbol);

        // Determine FillEvent context based on existence or absence of an open Position
        match self.repository.remove_position(&position_id)? {
            // EXIT SCENARIO - FillEvent for Symbol-Exchange combination with open Position
            Some(mut position) => {

                // Exit Position (in place mutation), & add the PositionExit event to Vec<Event>
                let position_exit = position.exit(balance, fill)?;
                generated_events.push(Event::PositionExit(position_exit));

                // Update Portfolio balance on Position exit
                // '--> available balance adds enter_total_fees since included in result PnL calc
                balance.available += position.enter_value_gross
                    + position.realised_profit_loss
                    + position.enter_fees_total;
                balance.total += position.realised_profit_loss;

                // Update statistics for exited Position market
                let market_id = determine_market_id(fill.exchange, &fill.symbol);
                let mut stats = self.repository.get_statistics(&market_id)?;
                stats.update(&position);

                // Persist exited Position & Updated Market statistics in Repository
                self.repository.set_statistics(&market_id, stats)?;
                self.repository.set_exited_position(self.engine_id, position)?;
            }

            // ENTRY SCENARIO - FillEvent for Symbol-Exchange with no Position
            None => {
                // Enter new Position, & add the PositionNew event to Vec<Event>
                let position = Position::enter(self.engine_id, fill)?;
                generated_events.push(Event::PositionNew(position.clone()));

                // Update Portfolio Balance.available on Position entry
                balance.available += -position.enter_value_gross - position.enter_fees_total;

                // Add to current Positions in Repository
                self.repository.set_open_position(position)?;
            }
        };

        // Add new Balance event to the Vec<Event>
        generated_events.push(Event::Balance(balance));

        // Persist updated Portfolio Balance in Repository
        self.repository.set_balance(self.engine_id, balance)?;

        Ok(generated_events)
    }
}

impl<Repository, Allocator, RiskManager, Statistic> PositionHandler
    for MetaPortfolio<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    fn set_open_position(&mut self, position: Position) -> Result<(), RepositoryError> {
        self.repository.set_open_position(position)
    }

    fn get_open_position(
        &mut self,
        position_id: &PositionId,
    ) -> Result<Option<Position>, RepositoryError> {
        self.repository.get_open_position(position_id)
    }

    fn get_open_positions<'a, Markets: Iterator<Item = &'a Market>>(
        &mut self,
        _: Uuid,
        markets: Markets,
    ) -> Result<Vec<Position>, RepositoryError> {
        self.repository.get_open_positions(self.engine_id, markets)
    }

    fn remove_position(
        &mut self,
        position_id: &PositionId,
    ) -> Result<Option<Position>, RepositoryError> {
        self.repository.remove_position(position_id)
    }

    fn set_exited_position(&mut self, _: Uuid, position: Position) -> Result<(), RepositoryError> {
        self.repository
            .set_exited_position(self.engine_id, position)
    }

    fn get_exited_positions(&mut self, _: Uuid) -> Result<Vec<Position>, RepositoryError> {
        self.repository.get_exited_positions(self.engine_id)
    }
}

impl<Repository, Allocator, RiskManager, Statistic> StatisticHandler<Statistic>
    for MetaPortfolio<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    fn set_statistics(
        &mut self,
        market_id: &MarketId,
        statistic: Statistic,
    ) -> Result<(), RepositoryError> {
        self.repository.set_statistics(market_id, statistic)
    }

    fn get_statistics(&mut self, market_id: &MarketId) -> Result<Statistic, RepositoryError> {
        self.repository.get_statistics(market_id)
    }
}

impl<Repository, Allocator, RiskManager, Statistic>
    MetaPortfolio<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    /// Constructs a new [`MetaPortfolio`] using the provided [`PortfolioLego`] components, and
    /// persists the initial [`MetaPortfolio`] state in the Repository.
    pub fn init(
        lego: PortfolioLego<Repository, Allocator, RiskManager, Statistic>,
    ) -> Result<Self, PortfolioError> {
        // Construct MetaPortfolio instance
        let mut portfolio = Self {
            engine_id: lego.engine_id,
            repository: lego.repository,
            allocation_manager: lego.allocator,
            risk_manager: lego.risk,
            _statistic_marker: PhantomData::default(),
        };

        // Persist initial state in the repository
        portfolio.bootstrap_repository(lego.starting_cash, lego.markets, lego.statistic_config)?;

        Ok(portfolio)
    }

    /// Persist initial [`MetaPortfolio`] state in the repository. This includes initialised
    /// Statistics every market provided, as well as starting `AvailableCash` & `TotalEquity`.
    pub fn bootstrap_repository<Markets: IntoIterator<Item = Market>>(
        &mut self,
        starting_cash: f64,
        markets: Markets,
        statistic_config:
        Statistic::Config
    ) -> Result<(), PortfolioError> {
        // Persist initial Balance (total & available)
        self.repository.set_balance(self.engine_id, Balance {
            timestamp: Utc::now(),
            total: starting_cash,
            available: starting_cash
        })?;

        // Persist initial MetaPortfolio Statistics for every Market
        markets
            .into_iter()
            .try_for_each(|market| {
                self.repository
                    .set_statistics(&market.market_id(), Statistic::init(statistic_config))
                    .map_err(PortfolioError::RepositoryInteractionError)
            })
    }

    /// Returns a [`MetaPortfolioBuilder`] instance.
    pub fn builder() -> MetaPortfolioBuilder<Repository, Allocator, RiskManager, Statistic> {
        MetaPortfolioBuilder::new()
    }

    /// Determines if the Portfolio has any cash to enter a new [`Position`].
    fn no_cash_to_enter_new_position(&mut self) -> Result<bool, PortfolioError> {
        self.repository
            .get_balance(self.engine_id)
            .map(|balance| balance.available == 0.0)
            .map_err(PortfolioError::RepositoryInteractionError)
    }
}

#[derive(Debug, Default)]
pub struct MetaPortfolioBuilder<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    engine_id: Option<Uuid>,
    markets: Option<Vec<Market>>,
    starting_cash: Option<f64>,
    repository: Option<Repository>,
    allocation_manager: Option<Allocator>,
    risk_manager: Option<RiskManager>,
    statistic_config: Option<Statistic::Config>,
    _statistic_marker: Option<PhantomData<Statistic>>,
}

impl<Repository, Allocator, RiskManager, Statistic>
    MetaPortfolioBuilder<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    pub fn new() -> Self {
        Self {
            engine_id: None,
            markets: None,
            starting_cash: None,
            repository: None,
            allocation_manager: None,
            risk_manager: None,
            statistic_config: None,
            _statistic_marker: None,
        }
    }

    pub fn engine_id(self, value: Uuid) -> Self {
        Self {
            engine_id: Some(value),
            ..self
        }
    }

    pub fn markets(self, value: Vec<Market>) -> Self {
        Self {
            markets: Some(value),
            ..self
        }
    }

    pub fn starting_cash(self, value: f64) -> Self {
        Self {
            starting_cash: Some(value),
            ..self
        }
    }

    pub fn repository(self, value: Repository) -> Self {
        Self {
            repository: Some(value),
            ..self
        }
    }

    pub fn allocation_manager(self, value: Allocator) -> Self {
        Self {
            allocation_manager: Some(value),
            ..self
        }
    }

    pub fn risk_manager(self, value: RiskManager) -> Self {
        Self {
            risk_manager: Some(value),
            ..self
        }
    }

    pub fn statistic_config(self, value: Statistic::Config) -> Self {
        Self {
            statistic_config: Some(value),
            ..self
        }
    }

    pub fn build_and_init(
        self,
    ) -> Result<MetaPortfolio<Repository, Allocator, RiskManager, Statistic>, PortfolioError> {
        // Construct Portfolio
        let mut portfolio = MetaPortfolio {
            engine_id: self.engine_id.ok_or(PortfolioError::BuilderIncomplete)?,
            repository: self.repository.ok_or(PortfolioError::BuilderIncomplete)?,
            allocation_manager: self.allocation_manager.ok_or(PortfolioError::BuilderIncomplete)?,
            risk_manager: self.risk_manager.ok_or(PortfolioError::BuilderIncomplete)?,
            _statistic_marker: PhantomData::default(),
        };

        // Persist initial state in the Repository
        portfolio.bootstrap_repository(
            self.starting_cash.ok_or(PortfolioError::BuilderIncomplete)?,
            self.markets.ok_or(PortfolioError::BuilderIncomplete)?,
            self.statistic_config.ok_or(PortfolioError::BuilderIncomplete)?
        )?;

        Ok(portfolio)
    }
}

/// Parses an incoming [`SignalEvent`]'s signals map. Determines what the net signal [`Decision`]
/// will be, and it's associated [`SignalStrength`].
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
pub mod tests {
    use super::*;

    use crate::{Market, MarketId};
    use crate::test_util::{fill_event, market_event, position, signal_event};
    use crate::execution::Fees;
    use crate::strategy::SignalForceExit;
    use crate::portfolio::allocator::DefaultAllocator;
    use crate::portfolio::position::PositionBuilder;
    use crate::portfolio::repository::error::RepositoryError;
    use crate::portfolio::risk::DefaultRisk;
    use crate::statistic::summary::pnl::PnLReturnSummary;
    use barter_data::model::MarketData;

    #[derive(Default)]
    struct MockRepository<Statistic> {
        set_open_position: Option<fn(position: Position) -> Result<(), RepositoryError>>,
        get_open_position:
            Option<fn(position_id: &String) -> Result<Option<Position>, RepositoryError>>,
        get_open_positions: Option<
            fn(engine_id: Uuid, markets: Vec<&Market>) -> Result<Vec<Position>, RepositoryError>,
        >,
        remove_position:
            Option<fn(engine_id: &String) -> Result<Option<Position>, RepositoryError>>,
        set_exited_position:
            Option<fn(engine_id: Uuid, position: Position) -> Result<(), RepositoryError>>,
        get_exited_positions:
            Option<fn(engine_id: Uuid) -> Result<Vec<Position>, RepositoryError>>,
        set_balance: Option<fn(engine_id: Uuid, balance: Balance) -> Result<(), RepositoryError>>,
        get_balance: Option<fn(engine_id: Uuid) -> Result<Balance, RepositoryError>>,
        set_statistics:
            Option<fn(market_id: &MarketId, statistic: Statistic) -> Result<(), RepositoryError>>,
        get_statistics: Option<fn(market_id: &MarketId) -> Result<Statistic, RepositoryError>>,
        position: Option<PositionBuilder>,
        balance: Option<Balance>,
    }

    impl<Statistic> PositionHandler for MockRepository<Statistic> {
        fn set_open_position(&mut self, position: Position) -> Result<(), RepositoryError> {
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
                    .unrealised_profit_loss(position.unrealised_profit_loss)
                    .realised_profit_loss(position.realised_profit_loss),
            );
            self.set_open_position.unwrap()(position)
        }

        fn get_open_position(
            &mut self,
            position_id: &String,
        ) -> Result<Option<Position>, RepositoryError> {
            self.get_open_position.unwrap()(position_id)
        }

        fn get_open_positions<'a, Markets: Iterator<Item = &'a Market>>(
            &mut self,
            engine_id: Uuid,
            markets: Markets,
        ) -> Result<Vec<Position>, RepositoryError> {
            self.get_open_positions.unwrap()(engine_id, markets.into_iter().collect())
        }

        fn remove_position(
            &mut self,
            position_id: &String,
        ) -> Result<Option<Position>, RepositoryError> {
            self.remove_position.unwrap()(position_id)
        }

        fn set_exited_position(
            &mut self,
            portfolio_id: Uuid,
            position: Position,
        ) -> Result<(), RepositoryError> {
            self.set_exited_position.unwrap()(portfolio_id, position)
        }

        fn get_exited_positions(
            &mut self,
            portfolio_id: Uuid,
        ) -> Result<Vec<Position>, RepositoryError> {
            self.get_exited_positions.unwrap()(portfolio_id)
        }
    }

    impl<Statistic> BalanceHandler for MockRepository<Statistic> {
        fn set_balance(&mut self, engine_id: Uuid, balance: Balance) -> Result<(), RepositoryError> {
            self.balance = Some(balance);
            self.set_balance.unwrap()(engine_id, balance)
        }

        fn get_balance(&mut self, engine_id: Uuid) -> Result<Balance, RepositoryError> {
            self.get_balance.unwrap()(engine_id)
        }
    }

    impl<Statistic> StatisticHandler<Statistic> for MockRepository<Statistic> {
        fn set_statistics(
            &mut self,
            market_id: &MarketId,
            statistic: Statistic,
        ) -> Result<(), RepositoryError> {
            self.set_statistics.unwrap()(market_id, statistic)
        }

        fn get_statistics(&mut self, market_id: &MarketId) -> Result<Statistic, RepositoryError> {
            self.get_statistics.unwrap()(market_id)
        }
    }

    fn new_mocked_portfolio<Repository, Statistic>(
        mock_repository: Repository,
    ) -> Result<MetaPortfolio<Repository, DefaultAllocator, DefaultRisk, Statistic>, PortfolioError>
    where
        Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
        Statistic: PositionSummariser + Initialiser,
    {
        let builder = MetaPortfolio::builder()
            .engine_id(Uuid::new_v4())
            .starting_cash(1000.0)
            .repository(mock_repository)
            .allocation_manager(DefaultAllocator {
                default_order_value: 100.0,
            })
            .risk_manager(DefaultRisk {});

        build_uninitialised_portfolio(builder)
    }

    fn build_uninitialised_portfolio<Repository, Statistic>(
        builder: MetaPortfolioBuilder<Repository, DefaultAllocator, DefaultRisk, Statistic>,
    ) -> Result<MetaPortfolio<Repository, DefaultAllocator, DefaultRisk, Statistic>, PortfolioError>
    where
        Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
        Statistic: PositionSummariser + Initialiser,
    {
        Ok(MetaPortfolio {
            engine_id: builder.engine_id.ok_or(PortfolioError::BuilderIncomplete)?,
            repository: builder.repository.ok_or(PortfolioError::BuilderIncomplete)?,
            allocation_manager: builder.allocation_manager.ok_or(PortfolioError::BuilderIncomplete)?,
            risk_manager: builder.risk_manager.ok_or(PortfolioError::BuilderIncomplete)?,
            _statistic_marker: Default::default(),
        })
    }

    fn new_signal_force_exit() -> SignalForceExit {
        SignalForceExit {
            event_type: SignalForceExit::FORCED_EXIT_SIGNAL,
            timestamp: Utc::now(),
            exchange: "binance",
            symbol: "eth_usdt".to_string(),
        }
    }

    #[test]
    fn update_from_market_with_long_position_increasing_in_value() {
        // Build Portfolio
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_open_position = Some(|_| {
            Ok(Some({
                let mut input_position = position();
                input_position.direction = Direction::Long;
                input_position.quantity = 1.0;
                input_position.enter_fees_total = 3.0;
                input_position.current_symbol_price = 100.0;
                input_position.current_value_gross = 100.0;
                input_position.unrealised_profit_loss = -3.0; // -3.0 from entry fees
                input_position
            }))
        });
        mock_repository.set_open_position = Some(|_| Ok(()));
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input MarketEvent
        let mut input_market = market_event();

        match input_market.data {
            // candle.close +100.0 on input_position.current_symbol_price
            MarketData::Candle(ref mut candle) => candle.close = 200.0,
            MarketData::Trade(ref mut trade) => trade.price = 200.0,
        };

        let result_pos_update = portfolio
            .update_from_market(&input_market)
            .unwrap()
            .unwrap();
        let updated_position = portfolio.repository.position.unwrap();

        assert_eq!(updated_position.current_symbol_price.unwrap(), 200.0);
        assert_eq!(updated_position.current_value_gross.unwrap(), 200.0);

        // Unreal PnL Long = current_value_gross - enter_value_gross - enter_fees_total*2
        assert_eq!(
            updated_position.unrealised_profit_loss.unwrap(),
            200.0 - 100.0 - 6.0
        );
        assert_eq!(
            result_pos_update.unrealised_profit_loss,
            200.0 - 100.0 - 6.0
        );
    }

    #[test]
    fn update_from_market_with_long_position_decreasing_in_value() {
        // Build Portfolio
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_open_position = Some(|_| {
            Ok(Some({
                let mut input_position = position();
                input_position.direction = Direction::Long;
                input_position.quantity = 1.0;
                input_position.enter_fees_total = 3.0;
                input_position.current_symbol_price = 100.0;
                input_position.current_value_gross = 100.0;
                input_position.unrealised_profit_loss = -3.0; // -3.0 from entry fees
                input_position
            }))
        });
        mock_repository.set_open_position = Some(|_| Ok(()));
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input MarketEvent
        let mut input_market = market_event();
        match input_market.data {
            // -50.0 on input_position.current_symbol_price
            MarketData::Candle(ref mut candle) => candle.close = 50.0,
            MarketData::Trade(ref mut trade) => trade.price = 50.0,
        };

        let result_pos_update = portfolio
            .update_from_market(&input_market)
            .unwrap()
            .unwrap();
        let updated_position = portfolio.repository.position.unwrap();

        assert_eq!(updated_position.current_symbol_price.unwrap(), 50.0);
        assert_eq!(updated_position.current_value_gross.unwrap(), 50.0);
        // Unreal PnL Long = current_value_gross - enter_value_gross - enter_fees_total*2
        assert_eq!(
            updated_position.unrealised_profit_loss.unwrap(),
            50.0 - 100.0 - 6.0
        );
        assert_eq!(result_pos_update.unrealised_profit_loss, 50.0 - 100.0 - 6.0);
    }

    #[test]
    fn update_from_market_with_short_position_increasing_in_value() {
        // Build Portfolio
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_open_position = Some(|_| {
            Ok(Some({
                let mut input_position = position();
                input_position.direction = Direction::Short;
                input_position.quantity = -1.0;
                input_position.enter_fees_total = 3.0;
                input_position.current_symbol_price = 100.0;
                input_position.current_value_gross = 100.0;
                input_position.unrealised_profit_loss = -3.0; // -3.0 from entry fees
                input_position
            }))
        });
        mock_repository.set_open_position = Some(|_| Ok(()));
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input MarketEvent
        let mut input_market = market_event();
        match input_market.data {
            // -50.0 on input_position.current_symbol_price
            MarketData::Candle(ref mut candle) => candle.close = 50.0,
            MarketData::Trade(ref mut trade) => trade.price = 50.0,
        };

        let result_pos_update = portfolio
            .update_from_market(&input_market)
            .unwrap()
            .unwrap();
        let updated_position = portfolio.repository.position.unwrap();

        assert_eq!(updated_position.current_symbol_price.unwrap(), 50.0);
        assert_eq!(updated_position.current_value_gross.unwrap(), 50.0);
        // Unreal PnL Short = enter_value_gross - current_value_gross - enter_fees_total*2
        assert_eq!(
            updated_position.unrealised_profit_loss.unwrap(),
            100.0 - 50.0 - 6.0
        );
        assert_eq!(result_pos_update.unrealised_profit_loss, 100.0 - 50.0 - 6.0);
    }

    #[test]
    fn update_from_market_with_short_position_decreasing_in_value() {
        // Build Portfolio
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_open_position = Some(|_| {
            Ok(Some({
                let mut input_position = position();
                input_position.direction = Direction::Short;
                input_position.quantity = -1.0;
                input_position.enter_fees_total = 3.0;
                input_position.current_symbol_price = 100.0;
                input_position.current_value_gross = 100.0;
                input_position.unrealised_profit_loss = -3.0; // -3.0 from entry fees
                input_position
            }))
        });
        mock_repository.set_open_position = Some(|_| Ok(()));
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input MarketEvent
        let mut input_market = market_event();

        match input_market.data {
            // +100.0 on input_position.current_symbol_price
            MarketData::Candle(ref mut candle) => candle.close = 200.0,
            MarketData::Trade(ref mut trade) => trade.price = 200.0,
        };

        let result_pos_update = portfolio
            .update_from_market(&input_market)
            .unwrap()
            .unwrap();
        let updated_position = portfolio.repository.position.unwrap();

        assert_eq!(updated_position.current_symbol_price.unwrap(), 200.0);
        assert_eq!(updated_position.current_value_gross.unwrap(), 200.0);
        // Unreal PnL Short = enter_value_gross - current_value_gross - enter_fees_total*2
        assert_eq!(
            updated_position.unrealised_profit_loss.unwrap(),
            100.0 - 200.0 - 6.0
        );
        assert_eq!(
            result_pos_update.unrealised_profit_loss,
            100.0 - 200.0 - 6.0
        );
    }

    #[test]
    fn generate_no_order_with_no_position_and_no_cash() {
        // Build Portfolio
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_open_position = Some(|_| Ok(None));
        mock_repository.get_balance = Some(|_| Ok(Balance {
            timestamp: Utc::now(),
            total: 100.0,
            available: 0.0
        }));
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input SignalEvent
        let input_signal = signal_event();

        let actual = portfolio.generate_order(&input_signal).unwrap();

        assert!(actual.is_none())
    }

    #[test]
    fn generate_no_order_with_position_and_no_cash() {
        // Build Portfolio
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_open_position = Some(|_| Ok(Some(position())));
        mock_repository.get_balance = Some(|_| Ok(Balance {
            timestamp: Utc::now(),
            total: 100.0,
            available: 0.0
        }));
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input SignalEvent
        let input_signal = signal_event();

        let actual = portfolio.generate_order(&input_signal).unwrap();

        assert!(actual.is_none())
    }

    #[test]
    fn generate_order_long_with_no_position_and_input_net_long_signal() {
        // Build Portfolio
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_open_position = Some(|_| Ok(None));
        mock_repository.get_balance = Some(|_| Ok(Balance {
            timestamp: Utc::now(),
            total: 100.0,
            available: 100.0
        }));
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input SignalEvent
        let mut input_signal = signal_event();
        input_signal.signals.insert(Decision::Long, 1.0);

        let actual = portfolio.generate_order(&input_signal).unwrap().unwrap();

        assert_eq!(actual.decision, Decision::Long)
    }

    #[test]
    fn generate_order_short_with_no_position_and_input_net_short_signal() {
        // Build Portfolio
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_open_position = Some(|_| Ok(None));
        mock_repository.get_balance = Some(|_| Ok(Balance {
            timestamp: Utc::now(),
            total: 100.0,
            available: 100.0
        }));
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input SignalEvent
        let mut input_signal = signal_event();
        input_signal.signals.insert(Decision::Short, 1.0);

        let actual = portfolio.generate_order(&input_signal).unwrap().unwrap();

        assert_eq!(actual.decision, Decision::Short)
    }

    #[test]
    fn generate_order_close_long_with_long_position_and_input_net_close_long_signal() {
        // Build Portfolio
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_open_position = Some(|_| {
            Ok(Some({
                let mut position = position();
                position.direction = Direction::Long;
                position
            }))
        });
        mock_repository.get_balance = Some(|_| Ok(Balance {
            timestamp: Utc::now(),
            total: 100.0,
            available: 100.0
        }));
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input SignalEvent
        let mut input_signal = signal_event();
        input_signal.signals.insert(Decision::CloseLong, 1.0);

        let actual = portfolio.generate_order(&input_signal).unwrap().unwrap();

        assert_eq!(actual.decision, Decision::CloseLong)
    }

    #[test]
    fn generate_order_close_short_with_short_position_and_input_net_close_short_signal() {
        // Build Portfolio
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_open_position = Some(|_| {
            Ok(Some({
                let mut position = position();
                position.direction = Direction::Short;
                position
            }))
        });
        mock_repository.get_balance = Some(|_| Ok(Balance {
            timestamp: Utc::now(),
            total: 100.0,
            available: 100.0
        }));
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input SignalEvent
        let mut input_signal = signal_event();
        input_signal.signals.insert(Decision::CloseShort, 1.0);

        let actual = portfolio.generate_order(&input_signal).unwrap().unwrap();

        assert_eq!(actual.decision, Decision::CloseShort)
    }

    #[test]
    fn generate_exit_order_with_long_position_open() {
        // Build Portfolio
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_open_position = Some(|_| {
            Ok(Some({
                let mut position = position();
                position.direction = Direction::Long;
                position.quantity = 100.0;
                position
            }))
        });
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input SignalEvent
        let input_signal = new_signal_force_exit();

        // Expect Ok(Some(OrderEvent))
        let actual = portfolio
            .generate_exit_order(input_signal)
            .unwrap()
            .unwrap();

        assert_eq!(actual.decision, Decision::CloseLong);
        assert_eq!(actual.quantity, -100.0);
        assert_eq!(actual.order_type, OrderType::Market)
    }

    #[test]
    fn generate_exit_order_with_short_position_open() {
        // Build Portfolio
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_open_position = Some(|_| {
            Ok(Some({
                let mut position = position();
                position.direction = Direction::Short;
                position.quantity = -100.0;
                position
            }))
        });
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input SignalEvent
        let input_signal = new_signal_force_exit();

        // Expect Ok(Some(OrderEvent))
        let actual = portfolio
            .generate_exit_order(input_signal)
            .unwrap()
            .unwrap();

        assert_eq!(actual.decision, Decision::CloseShort);
        assert_eq!(actual.quantity, 100.0);
        assert_eq!(actual.order_type, OrderType::Market)
    }

    #[test]
    fn generate_no_exit_order_when_no_open_position_to_exit() {
        // Build Portfolio
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_open_position = Some(|_| Ok(None));

        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input SignalEvent
        let input_signal = new_signal_force_exit();

        let actual = portfolio.generate_exit_order(input_signal).unwrap();
        assert!(actual.is_none());
    }

    #[test]
    fn update_from_fill_entering_long_position() {
        // Build Portfolio
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_balance = Some(|_| Ok(Balance {
            timestamp: Utc::now(),
            total: 200.0,
            available: 200.0
        }));
        mock_repository.remove_position = Some(|_| Ok(None));
        mock_repository.set_open_position = Some(|_| Ok(()));
        mock_repository.set_balance = Some(|_, _| Ok(()));
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input FillEvent
        let mut input_fill = fill_event();
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
        let updated_cash = updated_repository.balance.unwrap().available;

        assert!(result.is_ok());
        assert_eq!(entered_position.direction.unwrap(), Direction::Long);
        assert_eq!(entered_position.enter_value_gross.unwrap(), 100.0);
        assert_eq!(entered_position.enter_fees_total.unwrap(), 3.0);
        assert_eq!(updated_cash, 200.0 - 100.0 - 3.0); // cash += enter_value_gross - enter_fees
    }

    #[test]
    fn update_from_fill_entering_short_position() {
        // Build Portfolio
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_balance = Some(|_| Ok(Balance {
            timestamp: Utc::now(),
            total: 200.0,
            available: 200.0
        }));
        mock_repository.remove_position = Some(|_| Ok(None));
        mock_repository.set_open_position = Some(|_| Ok(()));
        mock_repository.set_balance = Some(|_, _| Ok(()));
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input FillEvent
        let mut input_fill = fill_event();
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
        let updated_cash = updated_repository.balance.unwrap().available;

        assert!(result.is_ok());
        assert_eq!(entered_position.direction.unwrap(), Direction::Short);
        assert_eq!(entered_position.enter_value_gross.unwrap(), 100.0);
        assert_eq!(entered_position.enter_fees_total.unwrap(), 3.0);
        assert_eq!(updated_cash, 200.0 - 100.0 - 3.0); // cash += enter_value_gross - enter_fees
    }

    #[test]
    fn update_from_fill_exiting_long_position_in_profit() {
        // Build Portfolio
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_balance = Some(|_| Ok(Balance {
            timestamp: Utc::now(),
            total: 200.0,
            available: 97.0
        }));
        mock_repository.remove_position = Some(|_| {
            Ok({
                Some({
                    let mut input_position = position();
                    input_position.direction = Direction::Long;
                    input_position.quantity = 1.0;
                    input_position.enter_fees_total = 3.0;
                    input_position.enter_value_gross = 100.0;
                    input_position
                })
            })
        });
        mock_repository.get_statistics = Some(|_| Ok(PnLReturnSummary::default()));
        mock_repository.set_statistics = Some(|_, _| Ok(()));
        mock_repository.set_exited_position = Some(|_, _| Ok(()));
        mock_repository.set_balance = Some(|_, _| Ok(()));
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input FillEvent
        let mut input_fill = fill_event();
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
        let updated_cash = updated_repository.balance.unwrap().available;
        let updated_value = updated_repository.balance.unwrap().total;

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
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_balance = Some(|_| Ok(Balance {
            timestamp: Utc::now(),
            total: 200.0,
            available: 97.0
        }));
        mock_repository.remove_position = Some(|_| {
            Ok({
                Some({
                    let mut input_position = position();
                    input_position.direction = Direction::Long;
                    input_position.quantity = 1.0;
                    input_position.enter_fees_total = 3.0;
                    input_position.enter_value_gross = 100.0;
                    input_position
                })
            })
        });
        mock_repository.get_statistics = Some(|_| Ok(PnLReturnSummary::default()));
        mock_repository.set_statistics = Some(|_, _| Ok(()));
        mock_repository.set_exited_position = Some(|_, _| Ok(()));
        mock_repository.set_balance = Some(|_, _| Ok(()));
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input FillEvent
        let mut input_fill = fill_event();
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
        let updated_cash = updated_repository.balance.unwrap().available;
        let updated_value = updated_repository.balance.unwrap().total;

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
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_balance = Some(|_| Ok(Balance {
            timestamp: Utc::now(),
            total: 200.0,
            available: 97.0
        }));
        mock_repository.remove_position = Some(|_| {
            Ok({
                Some({
                    let mut input_position = position();
                    input_position.direction = Direction::Short;
                    input_position.quantity = -1.0;
                    input_position.enter_fees_total = 3.0;
                    input_position.enter_value_gross = 100.0;
                    input_position
                })
            })
        });
        mock_repository.get_statistics = Some(|_| Ok(PnLReturnSummary::default()));
        mock_repository.set_statistics = Some(|_, _| Ok(()));
        mock_repository.set_exited_position = Some(|_, _| Ok(()));
        mock_repository.set_balance = Some(|_, _| Ok(()));
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input FillEvent
        let mut input_fill = fill_event();
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
        let updated_cash = updated_repository.balance.unwrap().available;
        let updated_value = updated_repository.balance.unwrap().total;

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
        let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
        mock_repository.get_balance = Some(|_| Ok(Balance {
            timestamp: Utc::now(),
            total: 200.0,
            available: 97.0
        }));
        mock_repository.remove_position = Some(|_| {
            Ok({
                Some({
                    let mut input_position = position();
                    input_position.direction = Direction::Short;
                    input_position.quantity = -1.0;
                    input_position.enter_fees_total = 3.0;
                    input_position.enter_value_gross = 100.0;
                    input_position
                })
            })
        });
        mock_repository.get_statistics = Some(|_| Ok(PnLReturnSummary::default()));
        mock_repository.set_statistics = Some(|_, _| Ok(()));
        mock_repository.set_exited_position = Some(|_, _| Ok(()));
        mock_repository.set_balance = Some(|_, _| Ok(()));
        let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

        // Input FillEvent
        let mut input_fill = fill_event();
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
        let updated_cash = updated_repository.balance.unwrap().available;
        let updated_value = updated_repository.balance.unwrap().total;

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
        let mut position = position();
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
        let mut position = position();
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
        let mut position = position();
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
        let mut position = position();
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
        let mut position = position();
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
        let mut position = position();
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
