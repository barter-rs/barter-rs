use crate::execution::fill::{FillEvent, Fees, FeeAmount};
use crate::portfolio::error::PortfolioError;
use crate::data::market::MarketEvent;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::strategy::signal::Decision;

/// Enters a new [Position].
pub trait PositionEnterer {
    /// Returns a new [Position], given an input [FillEvent].
    fn enter(fill: &FillEvent) -> Result<Position, PortfolioError>;
}

/// Updates an open [Position].
pub trait PositionUpdater {
    /// Updates an open [Position] using the latest input [MarketEvent].
    fn update(&mut self, market: &MarketEvent);
}

/// Exits an open [Position].
pub trait PositionExiter {
    /// Exits an open [Position], given the input [FillEvent] returned from a execution::handler.
    fn exit(&mut self, fill: &FillEvent) -> Result<(), PortfolioError>;
}

/// Direction of the [Position] when it was opened, Long or Short.
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    Long,
    Short,
}

impl Default for Direction {
    fn default() -> Self {
        Self::Long
    }
}

/// Change in [Position] market value caused by a Position.update().
pub type PositionValueChange = f64;

/// Data encapsulating an ongoing or closed Position..
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Trace UUID of the last event to trigger a [Position] update.
    pub last_update_trace_id: Uuid,

    /// Timestamp of the last event to trigger a [Position] update.
    pub last_update_timestamp: DateTime<Utc>,

    /// Exchange associated with this [Position] instance.
    pub exchange: String,

    /// Ticker symbol associated with this [Position] instance.
    pub symbol: String,

    /// Long or Short.
    pub direction: Direction,

    /// +ve or -ve quantity of symbol contracts opened.
    pub quantity: f64,

    /// All fees types incurred from entering a [Position], and their associated [FeeAmount].
    pub enter_fees: Fees,

    /// Total of enter_fees incurred. Sum of every [FeeAmount] in [Fees] when entering a [Position].
    pub enter_fees_total: FeeAmount,

    /// Enter average price excluding the entry_fees_total.
    pub enter_avg_price_gross: f64,

    /// abs(Quantity) * enter_avg_price_gross.
    pub enter_value_gross: f64,

    /// All fees types incurred from exiting a [Position], and their associated [FeeAmount].
    pub exit_fees: Fees,

    /// Total of exit_fees incurred. Sum of every [FeeAmount] in [Fees] when entering a [Position].
    pub exit_fees_total: FeeAmount,

    /// Exit average price excluding the exit_fees_total.
    pub exit_avg_price_gross: f64,

    /// abs(Quantity) * exit_avg_price_gross.
    pub exit_value_gross: f64,

    /// Symbol current close price.
    pub current_symbol_price: f64,

    /// abs(Quantity) * current_symbol_price.
    pub current_value_gross: f64,

    /// Unrealised P&L whilst Position open.
    pub unreal_profit_loss: f64,

    /// Realised P&L after Position closed.
    pub result_profit_loss: f64,
}

impl PositionEnterer for Position {
    fn enter(fill: &FillEvent) -> Result<Position, PortfolioError> {
        // Enter fees
        let enter_fees_total = fill.fees.calculate_total_fees();

        // Enter price
        let enter_avg_price_gross = Position::calculate_avg_price_gross(fill);

        // Unreal profit & loss
        let unreal_profit_loss = -enter_fees_total * 2.0;

        Ok(Position {
            last_update_trace_id: fill.trace_id,
            last_update_timestamp: fill.timestamp,
            exchange: fill.exchange.clone(),
            symbol: fill.symbol.clone(),
            direction: Position::parse_entry_direction(&fill)?,
            quantity: fill.quantity,
            enter_fees: fill.fees.clone(),
            enter_fees_total,
            enter_avg_price_gross,
            enter_value_gross: fill.fill_value_gross,
            exit_fees: Fees::default(),
            exit_fees_total: 0.0,
            exit_avg_price_gross: 0.0,
            exit_value_gross: 0.0,
            current_symbol_price: enter_avg_price_gross,
            current_value_gross: fill.fill_value_gross,
            unreal_profit_loss,
            result_profit_loss: 0.0,
        })
    }
}

impl PositionUpdater for Position {
    fn update(&mut self, market: &MarketEvent) {
        self.last_update_trace_id = market.trace_id;
        self.last_update_timestamp = market.timestamp;

        self.current_symbol_price = market.bar.close;

        // Market value gross
        self.current_value_gross = market.bar.close * self.quantity.abs();

        // Unreal profit & loss
        self.unreal_profit_loss = self.calculate_unreal_profit_loss();
    }
}

impl PositionExiter for Position {
    fn exit(&mut self, fill: &FillEvent) -> Result<(), PortfolioError> {
        if fill.decision.is_entry() {
            return Err(PortfolioError::CannotExitPositionWithEntryFill)
        }

        self.last_update_trace_id = fill.trace_id;
        self.last_update_timestamp = fill.timestamp;

        // Exit fees
        self.exit_fees = fill.fees.clone();
        self.exit_fees_total = fill.fees.calculate_total_fees();

        // Exit value & price
        self.exit_value_gross = fill.fill_value_gross;
        self.exit_avg_price_gross = Position::calculate_avg_price_gross(fill);

        // Result profit & loss
        self.result_profit_loss = self.calculate_result_profit_loss();
        self.unreal_profit_loss = self.result_profit_loss;

        Ok(())
    }
}

impl Default for Position {
    fn default() -> Self {
        Self {
            last_update_trace_id: Uuid::new_v4(),
            last_update_timestamp: Utc::now(),
            exchange: String::from("BINANCE"),
            symbol: String::from("ETH-USD"),
            direction: Direction::default(),
            quantity: 1.0,
            enter_fees: Default::default(),
            enter_fees_total: 0.0,
            enter_avg_price_gross: 100.0,
            enter_value_gross: 100.0,
            exit_fees: Default::default(),
            exit_fees_total: 0.0,
            exit_avg_price_gross: 0.0,
            exit_value_gross: 0.0,
            current_symbol_price: 100.0,
            current_value_gross: 100.0,
            unreal_profit_loss: 0.0,
            result_profit_loss: 0.0
        }
    }
}

impl Position {
    /// Returns a [PositionBuilder] instance.
    pub fn builder() -> PositionBuilder {
        PositionBuilder::new()
    }

    /// Calculates the [Position::enter_avg_price_gross] or [Position::exit_avg_price_gross] of
    /// a [FillEvent].
    pub fn calculate_avg_price_gross(fill: &FillEvent) -> f64 {
        (fill.fill_value_gross / fill.quantity).abs()
    }

    /// Determine the [Position] entry [Direction] by analysing the input [FillEvent].
    pub fn parse_entry_direction(fill: &FillEvent) -> Result<Direction, PortfolioError> {
        match fill.decision {
            Decision::Long if fill.quantity.is_sign_positive() => Ok(Direction::Long),
            Decision::Short if fill.quantity.is_sign_negative() => Ok(Direction::Short),
            Decision::CloseLong | Decision::CloseShort => Err(PortfolioError::CannotEnterPositionWithExitFill),
            _ => Err(PortfolioError::ParseEntryDirectionError)
        }
    }

    /// Calculate the approximate [Position::unreal_profit_loss] of a [Position].
    pub fn calculate_unreal_profit_loss(&self) -> f64 {
        let approx_total_fees = self.enter_fees_total * 2.0;

        match self.direction {
            Direction::Long => self.current_value_gross - self.enter_value_gross - approx_total_fees,
            Direction::Short => self.enter_value_gross - self.current_value_gross - approx_total_fees,
        }
    }

    /// Calculate the exact [Position::result_profit_loss] of a [Position].
    pub fn calculate_result_profit_loss(&self) -> f64 {
        let total_fees = self.enter_fees_total + self.exit_fees_total;

        match self.direction {
            Direction::Long => self.exit_value_gross - self.enter_value_gross - total_fees,
            Direction::Short => self.enter_value_gross - self.exit_value_gross - total_fees,
        }
    }
}

/// Builder to construct [Position] instances.
pub struct PositionBuilder {
    pub last_update_trace_id: Option<Uuid>,
    pub last_update_timestamp: Option<DateTime<Utc>>,
    pub exchange: Option<String>,
    pub symbol: Option<String>,
    pub direction: Option<Direction>,
    pub quantity: Option<f64>,
    pub enter_fees: Option<Fees>,
    pub enter_fees_total: Option<FeeAmount>,
    pub enter_avg_price_gross: Option<f64>,
    pub enter_value_gross: Option<f64>,
    pub exit_fees: Option<Fees>,
    pub exit_fees_total: Option<FeeAmount>,
    pub exit_avg_price_gross: Option<f64>,
    pub exit_value_gross: Option<f64>,
    pub current_symbol_price: Option<f64>,
    pub current_value_gross: Option<f64>,
    pub unreal_profit_loss: Option<f64>,
    pub result_profit_loss: Option<f64>,
}

impl PositionBuilder {
    pub fn new() -> Self {
        Self {
            last_update_trace_id: None,
            last_update_timestamp: None,
            exchange: None,
            symbol: None,
            direction: None,
            quantity: None,
            enter_fees: None,
            enter_fees_total: None,
            enter_avg_price_gross: None,
            enter_value_gross: None,
            exit_fees: None,
            exit_fees_total: None,
            exit_avg_price_gross: None,
            exit_value_gross: None,
            current_symbol_price: None,
            current_value_gross: None,
            unreal_profit_loss: None,
            result_profit_loss: None
        }
    }

    pub fn last_update_trace_id(mut self, value: Uuid) -> Self {
        self.last_update_trace_id = Some(value);
        self
    }

    pub fn last_update_timestamp(mut self, value: DateTime<Utc>) -> Self {
        self.last_update_timestamp = Some(value);
        self
    }

    pub fn exchange(mut self, value: String) -> Self {
        self.exchange = Some(value);
        self
    }

    pub fn symbol(mut self, value: String) -> Self {
        self.symbol = Some(value);
        self
    }

    pub fn direction(mut self, value: Direction) -> Self {
        self.direction = Some(value);
        self
    }

    pub fn quantity(mut self, value: f64) -> Self {
        self.quantity = Some(value);
        self
    }

    pub fn enter_fees(mut self, value: Fees) -> Self {
        self.enter_fees = Some(value);
        self
    }

    pub fn enter_fees_total(mut self, value: FeeAmount) -> Self {
        self.enter_fees_total = Some(value);
        self
    }

    pub fn enter_avg_price_gross(mut self, value: f64) -> Self {
        self.enter_avg_price_gross = Some(value);
        self
    }

    pub fn enter_value_gross(mut self, value: f64) -> Self {
        self.enter_value_gross = Some(value);
        self
    }

    pub fn exit_fees(mut self, value: Fees) -> Self {
        self.exit_fees = Some(value);
        self
    }

    pub fn exit_fees_total(mut self, value: FeeAmount) -> Self {
        self.exit_fees_total = Some(value);
        self
    }

    pub fn exit_avg_price_gross(mut self, value: f64) -> Self {
        self.exit_avg_price_gross = Some(value);
        self
    }

    pub fn exit_value_gross(mut self, value: f64) -> Self {
        self.exit_value_gross = Some(value);
        self
    }

    pub fn current_symbol_price(mut self, value: f64) -> Self {
        self.current_symbol_price = Some(value);
        self
    }

    pub fn current_value_gross(mut self, value: f64) -> Self {
        self.current_value_gross = Some(value);
        self
    }

    pub fn unreal_profit_loss(mut self, value: f64) -> Self {
        self.unreal_profit_loss = Some(value);
        self
    }

    pub fn result_profit_loss(mut self, value: f64) -> Self {
        self.result_profit_loss = Some(value);
        self
    }

    pub fn build(self) -> Result<Position, PortfolioError> {
        if let (
            Some(last_update_trace_id),
            Some(last_update_timestamp),
            Some(exchange),
            Some(symbol),
            Some(direction),
            Some(quantity),
            Some(enter_fees),
            Some(enter_fees_total),
            Some(enter_avg_price_gross),
            Some(enter_value_gross),
            Some(exit_fees),
            Some(exit_fees_total),
            Some(exit_avg_price_gross),
            Some(exit_value_gross),
            Some(current_symbol_price),
            Some(current_value_gross),
            Some(unreal_profit_loss),
            Some(result_profit_loss),
        ) = (
            self.last_update_trace_id,
            self.last_update_timestamp,
            self.exchange,
            self.symbol,
            self.direction,
            self.quantity,
            self.enter_fees,
            self.enter_fees_total,
            self.enter_avg_price_gross,
            self.enter_value_gross,
            self.exit_fees,
            self.exit_fees_total,
            self.exit_avg_price_gross,
            self.exit_value_gross,
            self.current_symbol_price,
            self.current_value_gross,
            self.unreal_profit_loss,
            self.result_profit_loss,
        ) {
            Ok(Position {
                last_update_trace_id,
                last_update_timestamp,
                exchange,
                symbol,
                direction,
                quantity,
                enter_fees,
                enter_fees_total,
                enter_avg_price_gross,
                enter_value_gross,
                exit_fees,
                exit_fees_total,
                exit_avg_price_gross,
                exit_value_gross,
                current_symbol_price,
                current_value_gross,
                unreal_profit_loss,
                result_profit_loss
            })
        } else {
            Err(PortfolioError::BuilderIncomplete)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::signal::Decision;

    #[test]
    fn enter_new_position_with_long_decision_provided() {
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::Long;
        input_fill.quantity = 1.0;
        input_fill.fill_value_gross = 100.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        let position = Position::enter(&input_fill).unwrap();

        assert_eq!(position.direction, Direction::Long);
        assert_eq!(position.quantity, input_fill.quantity);
        assert_eq!(position.enter_fees_total, 3.0);
        assert_eq!(position.enter_fees.exchange, input_fill.fees.exchange);
        assert_eq!(position.enter_fees.slippage, input_fill.fees.slippage);
        assert_eq!(position.enter_fees.network, input_fill.fees.network);
        assert_eq!(position.enter_avg_price_gross, (input_fill.fill_value_gross / input_fill.quantity.abs()));
        assert_eq!(position.enter_value_gross, input_fill.fill_value_gross);
        assert_eq!(position.exit_fees_total, 0.0);
        assert_eq!(position.exit_avg_price_gross, 0.0);
        assert_eq!(position.exit_value_gross, 0.0);
        assert_eq!(position.current_symbol_price, (input_fill.fill_value_gross / input_fill.quantity.abs()));
        assert_eq!(position.current_value_gross, input_fill.fill_value_gross);
        assert_eq!(position.unreal_profit_loss, -6.0); // -2 * enter_fees_total
        assert_eq!(position.result_profit_loss, 0.0);
    }

    #[test]
    fn enter_new_position_with_short_decision_provided() {
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::Short;
        input_fill.quantity = -1.0;
        input_fill.fill_value_gross = 100.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        let position = Position::enter(&input_fill).unwrap();

        assert_eq!(position.direction, Direction::Short);
        assert_eq!(position.quantity, input_fill.quantity);
        assert_eq!(position.enter_fees_total, 3.0);
        assert_eq!(position.enter_fees.exchange, input_fill.fees.exchange);
        assert_eq!(position.enter_fees.slippage, input_fill.fees.slippage);
        assert_eq!(position.enter_fees.network, input_fill.fees.network);
        assert_eq!(position.enter_avg_price_gross, (input_fill.fill_value_gross / input_fill.quantity.abs()));
        assert_eq!(position.enter_value_gross, input_fill.fill_value_gross);
        assert_eq!(position.exit_fees_total, 0.0);
        assert_eq!(position.exit_avg_price_gross, 0.0);
        assert_eq!(position.exit_value_gross, 0.0);
        assert_eq!(position.current_symbol_price, (input_fill.fill_value_gross / input_fill.quantity.abs()));
        assert_eq!(position.current_value_gross, input_fill.fill_value_gross);
        assert_eq!(position.unreal_profit_loss, -6.0); // -2 * enter_fees_total
        assert_eq!(position.result_profit_loss, 0.0);
    }

    #[test]
    fn enter_new_position_and_return_err_with_close_long_decision_provided() -> Result<(), String> {
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::CloseLong;
        input_fill.quantity = -1.0;
        input_fill.fill_value_gross = 100.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        if let Err(_) = Position::enter(&input_fill) {
            Ok(())
        }
        else {
            Err(String::from("Position::enter did not return an Err and it should have."))
        }
    }

    #[test]
    fn enter_new_position_and_return_err_with_close_short_decision_provided() -> Result<(), String> {
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::CloseShort;
        input_fill.quantity = 1.0;
        input_fill.fill_value_gross = 100.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        if let Err(_) = Position::enter(&input_fill) {
            Ok(())
        }
        else {
            Err(String::from("Position::enter did not return an Err and it should have."))
        }
    }

    #[test]
    fn enter_new_position_and_return_err_with_negative_quantity_long_decision_provided() -> Result<(), String> {
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::Long;
        input_fill.quantity = -1.0;
        input_fill.fill_value_gross = 100.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        if let Err(_) = Position::enter(&input_fill) {
            Ok(())
        }
        else {
            Err(String::from("Position::enter did not return an Err and it should have."))
        }
    }

    #[test]
    fn enter_new_position_and_return_err_with_positive_quantity_short_decision_provided() -> Result<(), String> {
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::Short;
        input_fill.quantity = 1.0;
        input_fill.fill_value_gross = 100.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        if let Err(_) = Position::enter(&input_fill) {
            Ok(())
        }
        else {
            Err(String::from("Position::enter did not return an Err and it should have."))
        }
    }

    #[test]
    fn update_long_position_so_unreal_pnl_increases() {
        // Initial Position
        let mut position = Position::default();
        position.direction = Direction::Long;
        position.quantity = 1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unreal_profit_loss = position.enter_fees_total * -2.0;

        // Input MarketEvent
        let mut input_market = MarketEvent::default();
        input_market.bar.close = 200.0; // +100.0 higher than current_symbol_price

        // Update Position
        position.update(&input_market);

        // Assert update hasn't changed fields that are constant after creation
        assert_eq!(position.direction, Direction::Long);
        assert_eq!(position.quantity, 1.0);
        assert_eq!(position.enter_fees_total, 3.0);
        assert_eq!(position.enter_fees.exchange, 1.0);
        assert_eq!(position.enter_fees.slippage, 1.0);
        assert_eq!(position.enter_fees.network, 1.0);
        assert_eq!(position.enter_avg_price_gross, 100.0);
        assert_eq!(position.enter_value_gross, 100.0);

        // Assert updated fields are correct
        assert_eq!(position.current_symbol_price, input_market.bar.close);
        assert_eq!(position.current_value_gross, input_market.bar.close * position.quantity.abs());

        // current_value_gross - enter_value_gross - approx_total_fees
        assert_eq!(position.unreal_profit_loss, (200.0 - 100.0 - 6.0));
    }

    #[test]
    fn update_long_position_so_unreal_pnl_decreases() {
        // Initial Position
        let mut position = Position::default();
        position.direction = Direction::Long;
        position.quantity = 1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unreal_profit_loss = position.enter_fees_total * -2.0;

        // Input MarketEvent
        let mut input_market = MarketEvent::default();
        input_market.bar.close = 50.0; // -50.0 lower than current_symbol_price

        // Update Position & return the PositionValueChange
        position.update(&input_market);

        // Assert update hasn't changed fields that are constant after creation
        assert_eq!(position.direction, Direction::Long);
        assert_eq!(position.quantity, 1.0);
        assert_eq!(position.enter_fees_total, 3.0);
        assert_eq!(position.enter_fees.exchange, 1.0);
        assert_eq!(position.enter_fees.slippage, 1.0);
        assert_eq!(position.enter_fees.network, 1.0);
        assert_eq!(position.enter_avg_price_gross, 100.0);
        assert_eq!(position.enter_value_gross, 100.0);

        // Assert updated fields are correct
        assert_eq!(position.current_symbol_price, input_market.bar.close);
        assert_eq!(position.current_value_gross, input_market.bar.close * position.quantity.abs());

        // current_value_gross - enter_value_gross - approx_total_fees
        assert_eq!(position.unreal_profit_loss, (50.0 - 100.0 - 6.0));
    }

    #[test]
    fn update_short_position_so_unreal_pnl_increases() {
        // Initial Position
        let mut position = Position::default();
        position.direction = Direction::Short;
        position.quantity = -1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unreal_profit_loss = position.enter_fees_total * -2.0;

        // Input MarketEvent
        let mut input_market = MarketEvent::default();
        input_market.bar.close = 50.0; // -50.0 lower than current_symbol_price

        // Update Position
        position.update(&input_market);

        // Assert update hasn't changed fields that are constant after creation
        assert_eq!(position.direction, Direction::Short);
        assert_eq!(position.quantity, -1.0);
        assert_eq!(position.enter_fees_total, 3.0);
        assert_eq!(position.enter_fees.exchange, 1.0);
        assert_eq!(position.enter_fees.slippage, 1.0);
        assert_eq!(position.enter_fees.network, 1.0);
        assert_eq!(position.enter_avg_price_gross, 100.0);
        assert_eq!(position.enter_value_gross, 100.0);

        // Assert updated fields are correct
        assert_eq!(position.current_symbol_price, input_market.bar.close);
        assert_eq!(position.current_value_gross, input_market.bar.close * position.quantity.abs());

        // enter_value_gross - current_value_gross - approx_total_fees
        assert_eq!(position.unreal_profit_loss, (100.0 - 50.0 - 6.0));
    }

    #[test]
    fn update_short_position_so_unreal_pnl_decreases() {
        // Initial Position
        let mut position = Position::default();
        position.direction = Direction::Short;
        position.quantity = -1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unreal_profit_loss = position.enter_fees_total * -2.0;

        // Input MarketEvent
        let mut input_market = MarketEvent::default();
        input_market.bar.close = 200.0; // +100.0 higher than current_symbol_price

        // Update Position
        position.update(&input_market);

        // Assert update hasn't changed fields that are constant after creation
        assert_eq!(position.direction, Direction::Short);
        assert_eq!(position.quantity, -1.0);
        assert_eq!(position.enter_fees_total, 3.0);
        assert_eq!(position.enter_fees.exchange, 1.0);
        assert_eq!(position.enter_fees.slippage, 1.0);
        assert_eq!(position.enter_fees.network, 1.0);
        assert_eq!(position.enter_avg_price_gross, 100.0);
        assert_eq!(position.enter_value_gross, 100.0);

        // Assert updated fields are correct
        assert_eq!(position.current_symbol_price, input_market.bar.close);
        assert_eq!(position.current_value_gross, input_market.bar.close * position.quantity.abs());

        // enter_value_gross - current_value_gross - approx_total_fees
        assert_eq!(position.unreal_profit_loss, (100.0 - 200.0 - 6.0));
    }

    #[test]
    fn exit_long_position_with_positive_real_pnl() {
        // Initial Position
        let mut position = Position::default();
        position.direction = Direction::Long;
        position.quantity = 1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unreal_profit_loss = position.enter_fees_total * -2.0;

        // Input FillEvent
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::CloseLong;
        input_fill.quantity = -position.quantity;
        input_fill.fill_value_gross = 200.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        // Exit Position
        position.exit(&input_fill).unwrap();

        // Assert exit hasn't changed fields that are constant after creation
        assert_eq!(position.direction, Direction::Long);
        assert_eq!(position.quantity, 1.0);
        assert_eq!(position.enter_fees_total, 3.0);
        assert_eq!(position.enter_fees.exchange, 1.0);
        assert_eq!(position.enter_fees.slippage, 1.0);
        assert_eq!(position.enter_fees.network, 1.0);
        assert_eq!(position.enter_avg_price_gross, 100.0);
        assert_eq!(position.enter_value_gross, 100.0);

        // Assert fields changed by exit are correct
        assert_eq!(position.exit_fees_total, 3.0);
        assert_eq!(position.exit_fees.exchange, 1.0);
        assert_eq!(position.exit_fees.slippage, 1.0);
        assert_eq!(position.exit_fees.network, 1.0);
        assert_eq!(position.exit_value_gross, input_fill.fill_value_gross);
        assert_eq!(position.exit_avg_price_gross, input_fill.fill_value_gross / input_fill.quantity.abs());

        // exit_value_gross - enter_value_gross - total_fees
        assert_eq!(position.result_profit_loss, (200.0 - 100.0 - 6.0));
        assert_eq!(position.unreal_profit_loss, (200.0 - 100.0 - 6.0));
    }

    #[test]
    fn exit_long_position_with_negative_real_pnl() {
        // Initial Position
        let mut position = Position::default();
        position.direction = Direction::Long;
        position.quantity = 1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unreal_profit_loss = position.enter_fees_total * -2.0;

        // Input FillEvent
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::CloseLong;
        input_fill.quantity = -position.quantity;
        input_fill.fill_value_gross = 50.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        // Exit Position
        position.exit(&input_fill).unwrap();

        // Assert exit hasn't changed fields that are constant after creation
        assert_eq!(position.direction, Direction::Long);
        assert_eq!(position.quantity, 1.0);
        assert_eq!(position.enter_fees_total, 3.0);
        assert_eq!(position.enter_fees.exchange, 1.0);
        assert_eq!(position.enter_fees.slippage, 1.0);
        assert_eq!(position.enter_fees.network, 1.0);
        assert_eq!(position.enter_avg_price_gross, 100.0);
        assert_eq!(position.enter_value_gross, 100.0);

        // Assert fields changed by exit are correct
        assert_eq!(position.exit_fees_total, 3.0);
        assert_eq!(position.exit_fees.exchange, 1.0);
        assert_eq!(position.exit_fees.slippage, 1.0);
        assert_eq!(position.exit_fees.network, 1.0);
        assert_eq!(position.exit_value_gross, input_fill.fill_value_gross);
        assert_eq!(position.exit_avg_price_gross, input_fill.fill_value_gross / input_fill.quantity.abs());

        // exit_value_gross - enter_value_gross - total_fees
        assert_eq!(position.result_profit_loss, (50.0 - 100.0 - 6.0));
        assert_eq!(position.unreal_profit_loss, (50.0 - 100.0 - 6.0));
    }

    #[test]
    fn exit_short_position_with_positive_real_pnl() {
        // Initial Position
        let mut position = Position::default();
        position.direction = Direction::Short;
        position.quantity = -1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unreal_profit_loss = position.enter_fees_total * -2.0;

        // Input FillEvent
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::CloseShort;
        input_fill.quantity = -position.quantity;
        input_fill.fill_value_gross = 50.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        // Exit Position
        position.exit(&input_fill).unwrap();

        // Assert exit hasn't changed fields that are constant after creation
        assert_eq!(position.direction, Direction::Short);
        assert_eq!(position.quantity, -1.0);
        assert_eq!(position.enter_fees_total, 3.0);
        assert_eq!(position.enter_fees.exchange, 1.0);
        assert_eq!(position.enter_fees.slippage, 1.0);
        assert_eq!(position.enter_fees.network, 1.0);
        assert_eq!(position.enter_avg_price_gross, 100.0);
        assert_eq!(position.enter_value_gross, 100.0);

        // Assert fields changed by exit are correct
        assert_eq!(position.exit_fees_total, 3.0);
        assert_eq!(position.exit_fees.exchange, 1.0);
        assert_eq!(position.exit_fees.slippage, 1.0);
        assert_eq!(position.exit_fees.network, 1.0);
        assert_eq!(position.exit_value_gross, input_fill.fill_value_gross);
        assert_eq!(position.exit_avg_price_gross, input_fill.fill_value_gross / input_fill.quantity.abs());

        // enter_value_gross - current_value_gross - approx_total_fees
        assert_eq!(position.result_profit_loss, (100.0 - 50.0 - 6.0));
        assert_eq!(position.unreal_profit_loss, (100.0 - 50.0 - 6.0));
    }

    #[test]
    fn exit_short_position_with_negative_real_pnl() {
        // Initial Position
        let mut position = Position::default();
        position.direction = Direction::Short;
        position.quantity = -1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unreal_profit_loss = position.enter_fees_total * -2.0;

        // Input FillEvent
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::CloseShort;
        input_fill.quantity = -position.quantity;
        input_fill.fill_value_gross = 200.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        // Exit Position
        position.exit(&input_fill).unwrap();

        // Assert exit hasn't changed fields that are constant after creation
        assert_eq!(position.direction, Direction::Short);
        assert_eq!(position.quantity, -1.0);
        assert_eq!(position.enter_fees_total, 3.0);
        assert_eq!(position.enter_fees.exchange, 1.0);
        assert_eq!(position.enter_fees.slippage, 1.0);
        assert_eq!(position.enter_fees.network, 1.0);
        assert_eq!(position.enter_avg_price_gross, 100.0);
        assert_eq!(position.enter_value_gross, 100.0);

        // Assert fields changed by exit are correct
        assert_eq!(position.exit_fees_total, 3.0);
        assert_eq!(position.exit_fees.exchange, 1.0);
        assert_eq!(position.exit_fees.slippage, 1.0);
        assert_eq!(position.exit_fees.network, 1.0);
        assert_eq!(position.exit_value_gross, input_fill.fill_value_gross);
        assert_eq!(position.exit_avg_price_gross, input_fill.fill_value_gross / input_fill.quantity.abs());

        // enter_value_gross - current_value_gross - approx_total_fees
        assert_eq!(position.result_profit_loss, (100.0 - 200.0 - 6.0));
        assert_eq!(position.unreal_profit_loss, (100.0 - 200.0 - 6.0));
    }

    #[test]
    fn exit_long_position_with_long_entry_fill_and_return_err() -> Result<(), String> {
        // Initial Position
        let mut position = Position::default();
        position.direction = Direction::Short;
        position.quantity = -1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unreal_profit_loss = position.enter_fees_total * -2.0;

        // Input FillEvent
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::Long;
        input_fill.quantity = position.quantity;
        input_fill.fill_value_gross = 200.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        // Exit Position
        if let Err(_) = position.exit(&input_fill) {
            Ok(())
        }
        else {
            Err(String::from("Position::exit did not return an Err and it should have."))
        }
    }

    #[test]
    fn exit_short_position_with_short_entry_fill_and_return_err() -> Result<(), String> {
        // Initial Position
        let mut position = Position::default();
        position.direction = Direction::Short;
        position.quantity = -1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unreal_profit_loss = position.enter_fees_total * -2.0;

        // Input FillEvent
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::Short;
        input_fill.quantity = -position.quantity;
        input_fill.fill_value_gross = 200.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0
        };

        // Exit Position
        if let Err(_) = position.exit(&input_fill) {
            Ok(())
        }
        else {
            Err(String::from("Position::exit did not return an Err and it should have."))
        }
    }

    #[test]
    fn calculate_avg_price_gross_correctly_with_positive_quantity() {
        let mut input_fill = FillEvent::default();
        input_fill.fill_value_gross = 1000.0;
        input_fill.quantity = 1.0;

        let actual = Position::calculate_avg_price_gross(&input_fill);

        assert_eq!(actual, 1000.0)
    }

    #[test]
    fn calculate_avg_price_gross_correctly_with_negative_quantity() {
        let mut input_fill = FillEvent::default();
        input_fill.fill_value_gross = 1000.0;
        input_fill.quantity = -1.0;

        let actual = Position::calculate_avg_price_gross(&input_fill);

        assert_eq!(actual, 1000.0)
    }

    #[test]
    fn parse_entry_direction_as_long_with_positive_quantity_long_decision_provided() {
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::Long;
        input_fill.quantity = 1.0;

        let actual = Position::parse_entry_direction(&input_fill).unwrap();

        assert_eq!(actual, Direction::Long)
    }

    #[test]
    fn parse_entry_direction_as_short_with_negative_quantity_short_decision_provided() {
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::Short;
        input_fill.quantity = -1.0;

        let actual = Position::parse_entry_direction(&input_fill).unwrap();

        assert_eq!(actual, Direction::Short)
    }

    #[test]
    fn parse_entry_direction_and_return_err_with_close_long_decision_provided() -> Result<(), String> {
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::CloseLong;
        input_fill.quantity = -1.0;

        if let Err(_) = Position::parse_entry_direction(&input_fill) {
            Ok(())
        }
        else {
            Err(String::from("parse_entry_direction() did not return an Err & it should."))
        }
    }

    #[test]
    fn parse_entry_direction_and_return_err_with_close_short_decision_provided() -> Result<(), String> {
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::CloseShort;
        input_fill.quantity = 1.0;

        if let Err(_) = Position::parse_entry_direction(&input_fill) {
            Ok(())
        }
        else {
            Err(String::from("parse_entry_direction() did not return an Err & it should."))
        }
    }

    #[test]
    fn parse_entry_direction_and_return_err_with_negative_quantity_long_decision_provided() -> Result<(), String> {
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::Long;
        input_fill.quantity = -1.0;

        if let Err(_) = Position::parse_entry_direction(&input_fill) {
            Ok(())
        }
        else {
            Err(String::from("parse_entry_direction() did not return an Err & it should."))
        }
    }

    #[test]
    fn parse_entry_direction_and_return_err_with_positive_quantity_short_decision_provided() -> Result<(), String> {
        let mut input_fill = FillEvent::default();
        input_fill.decision = Decision::Short;
        input_fill.quantity = 1.0;

        if let Err(_) = Position::parse_entry_direction(&input_fill) {
            Ok(())
        }
        else {
            Err(String::from("parse_entry_direction() did not return an Err & it should."))
        }
    }
}
