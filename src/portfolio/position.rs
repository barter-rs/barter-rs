use std::convert::TryFrom;

use barter_data::model::MarketData;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ExchangeId, SymbolId};
use crate::data::MarketEvent;
use crate::execution::{FeeAmount, Fees, FillEvent};
use crate::portfolio::Balance;
use crate::portfolio::error::PortfolioError;
use crate::strategy::Decision;

/// Enters a new [`Position`].
pub trait PositionEnterer {
    /// Returns a new [`Position`], given an input [`FillEvent`] & an associated engine_id.
    fn enter(engine_id: Uuid, fill: &FillEvent) -> Result<Position, PortfolioError>;
}

/// Updates an open [`Position`].
pub trait PositionUpdater {
    /// Updates an open [`Position`] using the latest input [`MarketEvent`], returning a
    /// [`PositionUpdate`] that communicates the open [`Position`]'s change in state.
    fn update(&mut self, market: &MarketEvent) -> PositionUpdate;
}

/// Exits an open [`Position`].
pub trait PositionExiter {
    /// Exits an open [`Position`], given the input Portfolio equity & the [`FillEvent`] returned
    /// from an Execution handler.
    fn exit(
        &mut self,
        balance: Balance,
        fill: &FillEvent,
    ) -> Result<PositionExit, PortfolioError>;
}

/// Communicates a String represents a unique [`Position`] identifier.
pub type PositionId = String;

/// Returns a unique identifier for a [`Position`] given an engine_Id, exchange & symbol.
pub fn determine_position_id(
    engine_id: Uuid,
    exchange: ExchangeId,
    symbol: &SymbolId,
) -> PositionId {
    format!("{}_trader_{}_{}_position", engine_id, exchange, symbol)
}

/// Data encapsulating the state of an ongoing or closed [`Position`].
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Position {
    /// Unique identifier for a [`Position`] generated from an engine_id, [`ExchangeId`] & [`SymbolId`].
    pub position_id: PositionId,

    /// Metadata detailing trace UUIDs, timestamps & equity associated with entering, updating & exiting.
    pub meta: PositionMeta,

    /// Exchange associated with this [`Position`] instance.
    pub exchange: String,

    /// Ticker symbol associated with this [`Position`] instance.
    pub symbol: SymbolId,

    /// Long or Short.
    pub direction: Direction,

    /// +ve or -ve quantity of symbol contracts opened.
    pub quantity: f64,

    /// All fees types incurred from entering a [`Position`], and their associated [`FeeAmount`].
    pub enter_fees: Fees,

    /// Total of enter_fees incurred. Sum of every [`FeeAmount`] in [`Fees`] when entering a [`Position`].
    pub enter_fees_total: FeeAmount,

    /// Enter average price excluding the entry_fees_total.
    pub enter_avg_price_gross: f64,

    /// abs(Quantity) * enter_avg_price_gross.
    pub enter_value_gross: f64,

    /// All fees types incurred from exiting a [`Position`], and their associated [`FeeAmount`].
    pub exit_fees: Fees,

    /// Total of exit_fees incurred. Sum of every [`FeeAmount`] in [`Fees`] when entering a [`Position`].
    pub exit_fees_total: FeeAmount,

    /// Exit average price excluding the exit_fees_total.
    pub exit_avg_price_gross: f64,

    /// abs(Quantity) * exit_avg_price_gross.
    pub exit_value_gross: f64,

    /// Symbol current close price.
    pub current_symbol_price: f64,

    /// abs(Quantity) * current_symbol_price.
    pub current_value_gross: f64,

    /// Unrealised P&L whilst the [`Position`] is open.
    pub unrealised_profit_loss: f64,

    /// Realised P&L after the [`Position`] has closed.
    pub realised_profit_loss: f64,
}

impl PositionEnterer for Position {
    fn enter(engine_id: Uuid, fill: &FillEvent) -> Result<Position, PortfolioError> {
        // Initialise Position Metadata
        let metadata = PositionMeta {
            enter_trace_id: fill.trace_id,
            enter_timestamp: fill.market_meta.timestamp,
            last_update_trace_id: fill.trace_id,
            last_update_timestamp: fill.timestamp,
            exit_balance: None,
        };

        // Enter fees
        let enter_fees_total = fill.fees.calculate_total_fees();

        // Enter price
        let enter_avg_price_gross = Position::calculate_avg_price_gross(fill);

        // Unreal profit & loss
        let unrealised_profit_loss = -enter_fees_total * 2.0;

        Ok(Position {
            position_id: determine_position_id(engine_id, fill.exchange, &fill.symbol),
            exchange: fill.exchange.to_owned(),
            symbol: fill.symbol.clone(),
            meta: metadata,
            direction: Position::parse_entry_direction(fill)?,
            quantity: fill.quantity,
            enter_fees: fill.fees,
            enter_fees_total,
            enter_avg_price_gross,
            enter_value_gross: fill.fill_value_gross,
            exit_fees: Fees::default(),
            exit_fees_total: 0.0,
            exit_avg_price_gross: 0.0,
            exit_value_gross: 0.0,
            current_symbol_price: enter_avg_price_gross,
            current_value_gross: fill.fill_value_gross,
            unrealised_profit_loss,
            realised_profit_loss: 0.0,
        })
    }
}

impl PositionUpdater for Position {
    fn update(&mut self, market: &MarketEvent) -> PositionUpdate {
        // Determine close from MarketData
        let close = match &market.data {
            MarketData::Trade(trade) => trade.price,
            MarketData::Candle(candle) => candle.close,
        };

        self.meta.last_update_trace_id = market.trace_id;
        self.meta.last_update_timestamp = market.timestamp;

        self.current_symbol_price = close;

        // Market value gross
        self.current_value_gross = close * self.quantity.abs();

        // Unreal profit & loss
        self.unrealised_profit_loss = self.calculate_unrealised_profit_loss();

        // Return a PositionUpdate event that communicates the change in state
        PositionUpdate::from(self)
    }
}

impl PositionExiter for Position {
    fn exit(
        &mut self,
        mut balance: Balance,
        fill: &FillEvent,
    ) -> Result<PositionExit, PortfolioError> {
        if fill.decision.is_entry() {
            return Err(PortfolioError::CannotExitPositionWithEntryFill);
        }

        // Exit fees
        self.exit_fees = fill.fees;
        self.exit_fees_total = fill.fees.calculate_total_fees();

        // Exit value & price
        self.exit_value_gross = fill.fill_value_gross;
        self.exit_avg_price_gross = Position::calculate_avg_price_gross(fill);

        // Result profit & loss
        self.realised_profit_loss = self.calculate_realised_profit_loss();
        self.unrealised_profit_loss = self.realised_profit_loss;

        // Metadata
        balance.total += self.realised_profit_loss;
        self.meta.last_update_trace_id = fill.trace_id;
        self.meta.last_update_timestamp = fill.timestamp;
        self.meta.exit_balance = Some(balance);

        PositionExit::try_from(self)
    }
}

impl Position {
    /// Returns a [`PositionBuilder`] instance.
    pub fn builder() -> PositionBuilder {
        PositionBuilder::new()
    }

    /// Calculates the [`Position::enter_avg_price_gross`] or [`Position::exit_avg_price_gross`] of
    /// a [`FillEvent`].
    pub fn calculate_avg_price_gross(fill: &FillEvent) -> f64 {
        (fill.fill_value_gross / fill.quantity).abs()
    }

    /// Determine the [`Position`] entry [`Direction`] by analysing the input [`FillEvent`].
    pub fn parse_entry_direction(fill: &FillEvent) -> Result<Direction, PortfolioError> {
        match fill.decision {
            Decision::Long if fill.quantity.is_sign_positive() => Ok(Direction::Long),
            Decision::Short if fill.quantity.is_sign_negative() => Ok(Direction::Short),
            Decision::CloseLong | Decision::CloseShort => {
                Err(PortfolioError::CannotEnterPositionWithExitFill)
            }
            _ => Err(PortfolioError::ParseEntryDirectionError),
        }
    }

    /// Calculate the approximate [`Position::unrealised_profit_loss`] of a [`Position`].
    pub fn calculate_unrealised_profit_loss(&self) -> f64 {
        let approx_total_fees = self.enter_fees_total * 2.0;

        match self.direction {
            Direction::Long => {
                self.current_value_gross - self.enter_value_gross - approx_total_fees
            }
            Direction::Short => {
                self.enter_value_gross - self.current_value_gross - approx_total_fees
            }
        }
    }

    /// Calculate the exact [`Position::realised_profit_loss`] of a [`Position`].
    pub fn calculate_realised_profit_loss(&self) -> f64 {
        let total_fees = self.enter_fees_total + self.exit_fees_total;

        match self.direction {
            Direction::Long => self.exit_value_gross - self.enter_value_gross - total_fees,
            Direction::Short => self.enter_value_gross - self.exit_value_gross - total_fees,
        }
    }

    /// Calculate the PnL return of a closed [`Position`] - assumed [`Position::realised_profit_loss`] is
    /// appropriately calculated.
    pub fn calculate_profit_loss_return(&self) -> f64 {
        self.realised_profit_loss / self.enter_value_gross
    }
}

/// Builder to construct [`Position`] instances.
#[derive(Debug, Default)]
pub struct PositionBuilder {
    pub position_id: Option<PositionId>,
    pub exchange: Option<String>,
    pub symbol: Option<SymbolId>,
    pub meta: Option<PositionMeta>,
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
    pub unrealised_profit_loss: Option<f64>,
    pub realised_profit_loss: Option<f64>,
}

impl PositionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn position_id(self, value: PositionId) -> Self {
        Self {
            position_id: Some(value),
            ..self
        }
    }

    pub fn exchange(self, value: String) -> Self {
        Self {
            exchange: Some(value),
            ..self
        }
    }

    pub fn symbol(self, value: SymbolId) -> Self {
        Self {
            symbol: Some(value),
            ..self
        }
    }

    pub fn meta(self, value: PositionMeta) -> Self {
        Self {
            meta: Some(value),
            ..self
        }
    }

    pub fn direction(self, value: Direction) -> Self {
        Self {
            direction: Some(value),
            ..self
        }
    }

    pub fn quantity(self, value: f64) -> Self {
        Self {
            quantity: Some(value),
            ..self
        }
    }

    pub fn enter_fees(self, value: Fees) -> Self {
        Self {
            enter_fees: Some(value),
            ..self
        }
    }

    pub fn enter_fees_total(self, value: FeeAmount) -> Self {
        Self {
            enter_fees_total: Some(value),
            ..self
        }
    }

    pub fn enter_avg_price_gross(self, value: f64) -> Self {
        Self {
            enter_avg_price_gross: Some(value),
            ..self
        }
    }

    pub fn enter_value_gross(self, value: f64) -> Self {
        Self {
            enter_value_gross: Some(value),
            ..self
        }
    }

    pub fn exit_fees(self, value: Fees) -> Self {
        Self {
            exit_fees: Some(value),
            ..self
        }
    }

    pub fn exit_fees_total(self, value: FeeAmount) -> Self {
        Self {
            exit_fees_total: Some(value),
            ..self
        }
    }

    pub fn exit_avg_price_gross(self, value: f64) -> Self {
        Self {
            exit_avg_price_gross: Some(value),
            ..self
        }
    }

    pub fn exit_value_gross(self, value: f64) -> Self {
        Self {
            exit_value_gross: Some(value),
            ..self
        }
    }

    pub fn current_symbol_price(self, value: f64) -> Self {
        Self {
            current_symbol_price: Some(value),
            ..self
        }
    }

    pub fn current_value_gross(self, value: f64) -> Self {
        Self {
            current_value_gross: Some(value),
            ..self
        }
    }

    pub fn unrealised_profit_loss(self, value: f64) -> Self {
        Self {
            unrealised_profit_loss: Some(value),
            ..self
        }
    }

    pub fn realised_profit_loss(self, value: f64) -> Self {
        Self {
            realised_profit_loss: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<Position, PortfolioError> {
        let position_id = self.position_id.ok_or(PortfolioError::BuilderIncomplete)?;
        let exchange = self.exchange.ok_or(PortfolioError::BuilderIncomplete)?;
        let symbol = self.symbol.ok_or(PortfolioError::BuilderIncomplete)?;
        let meta = self.meta.ok_or(PortfolioError::BuilderIncomplete)?;
        let direction = self.direction.ok_or(PortfolioError::BuilderIncomplete)?;
        let quantity = self.quantity.ok_or(PortfolioError::BuilderIncomplete)?;
        let enter_fees = self.enter_fees.ok_or(PortfolioError::BuilderIncomplete)?;
        let enter_fees_total = self
            .enter_fees_total
            .ok_or(PortfolioError::BuilderIncomplete)?;
        let enter_avg_price_gross = self
            .enter_avg_price_gross
            .ok_or(PortfolioError::BuilderIncomplete)?;
        let enter_value_gross = self
            .enter_value_gross
            .ok_or(PortfolioError::BuilderIncomplete)?;
        let exit_fees = self.exit_fees.ok_or(PortfolioError::BuilderIncomplete)?;
        let exit_fees_total = self
            .exit_fees_total
            .ok_or(PortfolioError::BuilderIncomplete)?;
        let exit_avg_price_gross = self
            .exit_avg_price_gross
            .ok_or(PortfolioError::BuilderIncomplete)?;
        let exit_value_gross = self
            .exit_value_gross
            .ok_or(PortfolioError::BuilderIncomplete)?;
        let current_symbol_price = self
            .current_symbol_price
            .ok_or(PortfolioError::BuilderIncomplete)?;
        let current_value_gross = self
            .current_value_gross
            .ok_or(PortfolioError::BuilderIncomplete)?;
        let unrealised_profit_loss = self
            .unrealised_profit_loss
            .ok_or(PortfolioError::BuilderIncomplete)?;
        let realised_profit_loss = self
            .realised_profit_loss
            .ok_or(PortfolioError::BuilderIncomplete)?;

        Ok(Position {
            position_id,
            exchange,
            symbol,
            meta,
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
            unrealised_profit_loss,
            realised_profit_loss,
        })
    }
}

/// Metadata detailing the trace UUIDs & timestamps associated with entering, updating & exiting
/// a [`Position`].
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PositionMeta {
    /// Trace UUID linking all the Events that led to the entering of this [`Position`].
    pub enter_trace_id: Uuid,

    /// FillEvent timestamp that triggered the entering of this [`Position`].
    pub enter_timestamp: DateTime<Utc>,

    /// Trace UUID of the last event to trigger a [`Position`] state change (enter, update, exit).
    pub last_update_trace_id: Uuid,

    /// Timestamp of the last event to trigger a [`Position`] state change (enter, update, exit).
    pub last_update_timestamp: DateTime<Utc>,

    /// Portfolio [`Balance`] calculated at the point of exiting a [`Position`].
    pub exit_balance: Option<Balance>,
}

impl Default for PositionMeta {
    fn default() -> Self {
        Self {
            enter_trace_id: Uuid::new_v4(),
            enter_timestamp: Utc::now(),
            last_update_trace_id: Uuid::new_v4(),
            last_update_timestamp: Utc::now(),
            exit_balance: None,
        }
    }
}

/// Direction of the [`Position`] when it was opened, Long or Short.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum Direction {
    Long,
    Short,
}

impl Default for Direction {
    fn default() -> Self {
        Self::Long
    }
}

impl Direction {
    /// Determines the [`Decision`] required to exit a [`Position`] that's in a specific [`Direction`].
    pub fn determine_exit_decision(&self) -> Decision {
        match self {
            Direction::Long => Decision::CloseLong,
            Direction::Short => Decision::CloseShort,
        }
    }
}

/// [`Position`] update event. Occurs as a result of receiving new [`MarketEvent`] data.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PositionUpdate {
    /// Unique identifier for a [`Position`], generated from an exchange, symbol, and enter_timestamp.
    pub position_id: String,
    /// Trace UUID of the last event to trigger a [`Position`] update.
    pub update_trace_id: Uuid,
    /// Event timestamp of the last event to trigger a [`Position`] update.
    pub update_timestamp: DateTime<Utc>,
    /// Symbol current close price.
    pub current_symbol_price: f64,
    /// abs(Quantity) * current_symbol_price.
    pub current_value_gross: f64,
    /// Unrealised P&L whilst the [`Position`] is open.
    pub unrealised_profit_loss: f64,
}

impl From<&mut Position> for PositionUpdate {
    fn from(updated_position: &mut Position) -> Self {
        Self {
            position_id: updated_position.position_id.clone(),
            update_trace_id: updated_position.meta.last_update_trace_id,
            update_timestamp: updated_position.meta.last_update_timestamp,
            current_symbol_price: updated_position.current_symbol_price,
            current_value_gross: updated_position.current_value_gross,
            unrealised_profit_loss: updated_position.unrealised_profit_loss,
        }
    }
}

/// [`Position`] exit event. Occurs as a result of a [`FillEvent`] that exits a [`Position`].
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PositionExit {
    /// Unique identifier for a [`Position`], generated from an exchange, symbol, and enter_timestamp.
    pub position_id: String,

    /// Trace UUID linking the last chain of events to trigger a [`Position`] exit.
    pub exit_trace_id: Uuid,

    /// FillEvent timestamp that triggered the exiting of this [`Position`].
    pub exit_timestamp: DateTime<Utc>,

    /// Portfolio [`Balance`] calculated at the point of exiting a [`Position`].
    pub exit_balance: Balance,

    /// All fees types incurred from exiting a [`Position`], and their associated [`FeeAmount`].
    pub exit_fees: Fees,

    /// Total of exit_fees incurred. Sum of every [`FeeAmount`] in [`Fees`] when entering a [`Position`].
    pub exit_fees_total: FeeAmount,

    /// Exit average price excluding the exit_fees_total.
    pub exit_avg_price_gross: f64,

    /// abs(Quantity) * exit_avg_price_gross.
    pub exit_value_gross: f64,

    /// Realised P&L after the [`Position`] has closed.
    pub realised_profit_loss: f64,
}

impl TryFrom<&mut Position> for PositionExit {
    type Error = PortfolioError;

    fn try_from(exited_position: &mut Position) -> Result<Self, Self::Error> {
        Ok(Self {
            position_id: exited_position.position_id.clone(),
            exit_trace_id: exited_position.meta.last_update_trace_id,
            exit_timestamp: exited_position.meta.last_update_timestamp,
            exit_balance: exited_position.meta.exit_balance.ok_or(PortfolioError::PositionExitError)?,
            exit_fees: exited_position.exit_fees,
            exit_fees_total: exited_position.exit_fees_total,
            exit_avg_price_gross: exited_position.exit_avg_price_gross,
            exit_value_gross: exited_position.exit_value_gross,
            realised_profit_loss: exited_position.realised_profit_loss,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{fill_event, market_event, position};

    #[test]
    fn enter_new_position_with_long_decision_provided() {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::Long;
        input_fill.quantity = 1.0;
        input_fill.fill_value_gross = 100.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };

        let position = Position::enter(Uuid::new_v4(), &input_fill).unwrap();

        assert_eq!(position.direction, Direction::Long);
        assert_eq!(position.quantity, input_fill.quantity);
        assert_eq!(position.enter_fees_total, 3.0);
        assert_eq!(position.enter_fees.exchange, input_fill.fees.exchange);
        assert_eq!(position.enter_fees.slippage, input_fill.fees.slippage);
        assert_eq!(position.enter_fees.network, input_fill.fees.network);
        assert_eq!(
            position.enter_avg_price_gross,
            (input_fill.fill_value_gross / input_fill.quantity.abs())
        );
        assert_eq!(position.enter_value_gross, input_fill.fill_value_gross);
        assert_eq!(position.exit_fees_total, 0.0);
        assert_eq!(position.exit_avg_price_gross, 0.0);
        assert_eq!(position.exit_value_gross, 0.0);
        assert_eq!(
            position.current_symbol_price,
            (input_fill.fill_value_gross / input_fill.quantity.abs())
        );
        assert_eq!(position.current_value_gross, input_fill.fill_value_gross);
        assert_eq!(position.unrealised_profit_loss, -6.0); // -2 * enter_fees_total
        assert_eq!(position.realised_profit_loss, 0.0);
    }

    #[test]
    fn enter_new_position_with_short_decision_provided() {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::Short;
        input_fill.quantity = -1.0;
        input_fill.fill_value_gross = 100.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };

        let position = Position::enter(Uuid::new_v4(), &input_fill).unwrap();

        assert_eq!(position.direction, Direction::Short);
        assert_eq!(position.quantity, input_fill.quantity);
        assert_eq!(position.enter_fees_total, 3.0);
        assert_eq!(position.enter_fees.exchange, input_fill.fees.exchange);
        assert_eq!(position.enter_fees.slippage, input_fill.fees.slippage);
        assert_eq!(position.enter_fees.network, input_fill.fees.network);
        assert_eq!(
            position.enter_avg_price_gross,
            (input_fill.fill_value_gross / input_fill.quantity.abs())
        );
        assert_eq!(position.enter_value_gross, input_fill.fill_value_gross);
        assert_eq!(position.exit_fees_total, 0.0);
        assert_eq!(position.exit_avg_price_gross, 0.0);
        assert_eq!(position.exit_value_gross, 0.0);
        assert_eq!(
            position.current_symbol_price,
            (input_fill.fill_value_gross / input_fill.quantity.abs())
        );
        assert_eq!(position.current_value_gross, input_fill.fill_value_gross);
        assert_eq!(position.unrealised_profit_loss, -6.0); // -2 * enter_fees_total
        assert_eq!(position.realised_profit_loss, 0.0);
    }

    #[test]
    fn enter_new_position_and_return_err_with_close_long_decision_provided() -> Result<(), String> {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::CloseLong;
        input_fill.quantity = -1.0;
        input_fill.fill_value_gross = 100.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };

        if let Err(_) = Position::enter(Uuid::new_v4(), &input_fill) {
            Ok(())
        } else {
            Err(String::from(
                "Position::enter did not return an Err and it should have.",
            ))
        }
    }

    #[test]
    fn enter_new_position_and_return_err_with_close_short_decision_provided() -> Result<(), String>
    {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::CloseShort;
        input_fill.quantity = 1.0;
        input_fill.fill_value_gross = 100.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };

        if let Err(_) = Position::enter(Uuid::new_v4(), &input_fill) {
            Ok(())
        } else {
            Err(String::from(
                "Position::enter did not return an Err and it should have.",
            ))
        }
    }

    #[test]
    fn enter_new_position_and_return_err_with_negative_quantity_long_decision_provided(
    ) -> Result<(), String> {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::Long;
        input_fill.quantity = -1.0;
        input_fill.fill_value_gross = 100.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };

        if let Err(_) = Position::enter(Uuid::new_v4(), &input_fill) {
            Ok(())
        } else {
            Err(String::from(
                "Position::enter did not return an Err and it should have.",
            ))
        }
    }

    #[test]
    fn enter_new_position_and_return_err_with_positive_quantity_short_decision_provided(
    ) -> Result<(), String> {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::Short;
        input_fill.quantity = 1.0;
        input_fill.fill_value_gross = 100.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };

        if let Err(_) = Position::enter(Uuid::new_v4(), &input_fill) {
            Ok(())
        } else {
            Err(String::from(
                "Position::enter did not return an Err and it should have.",
            ))
        }
    }

    #[test]
    fn update_long_position_so_unreal_pnl_increases() {
        // Initial Position
        let mut position = position();
        position.direction = Direction::Long;
        position.quantity = 1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unrealised_profit_loss = position.enter_fees_total * -2.0;

        // Input MarketEvent
        let mut input_market = market_event();
        match input_market.data {
            // +100.0 higher than current_symbol_price
            MarketData::Candle(ref mut candle) => candle.close = 200.0,
            MarketData::Trade(ref mut trade) => trade.price = 200.0,
        };

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
        let close = match &input_market.data {
            MarketData::Trade(trade) => trade.price,
            MarketData::Candle(candle) => candle.close,
        };
        assert_eq!(position.current_symbol_price, close);
        assert_eq!(
            position.current_value_gross,
            close * position.quantity.abs()
        );

        // current_value_gross - enter_value_gross - approx_total_fees
        assert_eq!(position.unrealised_profit_loss, (200.0 - 100.0 - 6.0));
    }

    #[test]
    fn update_long_position_so_unreal_pnl_decreases() {
        // Initial Position
        let mut position = position();
        position.direction = Direction::Long;
        position.quantity = 1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unrealised_profit_loss = position.enter_fees_total * -2.0;

        // Input MarketEvent
        let mut input_market = market_event();

        match input_market.data {
            // -50.0 lower than current_symbol_price
            MarketData::Candle(ref mut candle) => candle.close = 50.0,
            MarketData::Trade(ref mut trade) => trade.price = 50.0,
        };

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
        let close = match &input_market.data {
            MarketData::Trade(trade) => trade.price,
            MarketData::Candle(candle) => candle.close,
        };
        assert_eq!(position.current_symbol_price, close);
        assert_eq!(
            position.current_value_gross,
            close * position.quantity.abs()
        );

        // current_value_gross - enter_value_gross - approx_total_fees
        assert_eq!(position.unrealised_profit_loss, (50.0 - 100.0 - 6.0));
    }

    #[test]
    fn update_short_position_so_unreal_pnl_increases() {
        // Initial Position
        let mut position = position();
        position.direction = Direction::Short;
        position.quantity = -1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unrealised_profit_loss = position.enter_fees_total * -2.0;

        // Input MarketEvent
        let mut input_market = market_event();

        match input_market.data {
            // -50.0 lower than current_symbol_price
            MarketData::Candle(ref mut candle) => candle.close = 50.0,
            MarketData::Trade(ref mut trade) => trade.price = 50.0,
        };

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
        let close = match &input_market.data {
            MarketData::Trade(trade) => trade.price,
            MarketData::Candle(candle) => candle.close,
        };
        assert_eq!(position.current_symbol_price, close);
        assert_eq!(
            position.current_value_gross,
            close * position.quantity.abs()
        );

        // enter_value_gross - current_value_gross - approx_total_fees
        assert_eq!(position.unrealised_profit_loss, (100.0 - 50.0 - 6.0));
    }

    #[test]
    fn update_short_position_so_unreal_pnl_decreases() {
        // Initial Position
        let mut position = position();
        position.direction = Direction::Short;
        position.quantity = -1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unrealised_profit_loss = position.enter_fees_total * -2.0;

        // Input MarketEvent
        let mut input_market = market_event();

        match input_market.data {
            // +100.0 higher than current_symbol_price
            MarketData::Candle(ref mut candle) => candle.close = 200.0,
            MarketData::Trade(ref mut trade) => trade.price = 200.0,
        };

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
        let close = match &input_market.data {
            MarketData::Trade(trade) => trade.price,
            MarketData::Candle(candle) => candle.close,
        };
        assert_eq!(position.current_symbol_price, close);
        assert_eq!(
            position.current_value_gross,
            close * position.quantity.abs()
        );

        // enter_value_gross - current_value_gross - approx_total_fees
        assert_eq!(position.unrealised_profit_loss, (100.0 - 200.0 - 6.0));
    }

    #[test]
    fn exit_long_position_with_positive_real_pnl() {
        // Initial Position
        let mut position = position();
        position.direction = Direction::Long;
        position.quantity = 1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unrealised_profit_loss = position.enter_fees_total * -2.0;

        // Input Portfolio Current Balance
        let current_balance = Balance {
            timestamp: Utc::now(),
            total: 10000.0,
            available: 10000.0
        };

        // Input FillEvent
        let mut input_fill = fill_event();
        input_fill.decision = Decision::CloseLong;
        input_fill.quantity = -position.quantity;
        input_fill.fill_value_gross = 200.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };

        // Exit Position
        position.exit(current_balance, &input_fill).unwrap();

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
        assert_eq!(
            position.exit_avg_price_gross,
            input_fill.fill_value_gross / input_fill.quantity.abs()
        );

        // exit_value_gross - enter_value_gross - total_fees
        assert_eq!(position.realised_profit_loss, (200.0 - 100.0 - 6.0));
        assert_eq!(position.unrealised_profit_loss, (200.0 - 100.0 - 6.0));

        // Assert EquityPoint on Exit is correct
        assert_eq!(
            position.meta.exit_balance.unwrap().total,
            current_balance.total + (200.0 - 100.0 - 6.0)
        )
    }

    #[test]
    fn exit_long_position_with_negative_real_pnl() {
        // Initial Position
        let mut position = position();
        position.direction = Direction::Long;
        position.quantity = 1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unrealised_profit_loss = position.enter_fees_total * -2.0;

        // Input Portfolio Current Balance
        let current_balance = Balance {
            timestamp: Utc::now(),
            total: 10000.0,
            available: 10000.0
        };

        // Input FillEvent
        let mut input_fill = fill_event();
        input_fill.decision = Decision::CloseLong;
        input_fill.quantity = -position.quantity;
        input_fill.fill_value_gross = 50.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };

        // Exit Position
        position.exit(current_balance, &input_fill).unwrap();

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
        assert_eq!(
            position.exit_avg_price_gross,
            input_fill.fill_value_gross / input_fill.quantity.abs()
        );

        // exit_value_gross - enter_value_gross - total_fees
        assert_eq!(position.realised_profit_loss, (50.0 - 100.0 - 6.0));
        assert_eq!(position.unrealised_profit_loss, (50.0 - 100.0 - 6.0));

        // Assert EquityPoint on Exit is correct
        assert_eq!(
            position.meta.exit_balance.unwrap().total,
            current_balance.total + (50.0 - 100.0 - 6.0)
        )
    }

    #[test]
    fn exit_short_position_with_positive_real_pnl() {
        // Initial Position
        let mut position = position();
        position.direction = Direction::Short;
        position.quantity = -1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unrealised_profit_loss = position.enter_fees_total * -2.0;

        // Input Portfolio Current Balance
        let current_balance = Balance {
            timestamp: Utc::now(),
            total: 10000.0,
            available: 10000.0
        };

        // Input FillEvent
        let mut input_fill = fill_event();
        input_fill.decision = Decision::CloseShort;
        input_fill.quantity = -position.quantity;
        input_fill.fill_value_gross = 50.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };

        // Exit Position
        position.exit(current_balance, &input_fill).unwrap();

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
        assert_eq!(
            position.exit_avg_price_gross,
            input_fill.fill_value_gross / input_fill.quantity.abs()
        );

        // enter_value_gross - current_value_gross - approx_total_fees
        assert_eq!(position.realised_profit_loss, (100.0 - 50.0 - 6.0));
        assert_eq!(position.unrealised_profit_loss, (100.0 - 50.0 - 6.0));

        // Assert EquityPoint on Exit is correct
        assert_eq!(
            position.meta.exit_balance.unwrap().total,
            current_balance.total + (100.0 - 50.0 - 6.0)
        )
    }

    #[test]
    fn exit_short_position_with_negative_real_pnl() {
        // Initial Position
        let mut position = position();
        position.direction = Direction::Short;
        position.quantity = -1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unrealised_profit_loss = position.enter_fees_total * -2.0;

        // Input Portfolio Current Balance
        let current_balance = Balance {
            timestamp: Utc::now(),
            total: 10000.0,
            available: 10000.0
        };

        // Input FillEvent
        let mut input_fill = fill_event();
        input_fill.decision = Decision::CloseShort;
        input_fill.quantity = -position.quantity;
        input_fill.fill_value_gross = 200.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };

        // Exit Position
        position.exit(current_balance, &input_fill).unwrap();

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
        assert_eq!(
            position.exit_avg_price_gross,
            input_fill.fill_value_gross / input_fill.quantity.abs()
        );

        // enter_value_gross - current_value_gross - approx_total_fees
        assert_eq!(position.realised_profit_loss, (100.0 - 200.0 - 6.0));
        assert_eq!(position.unrealised_profit_loss, (100.0 - 200.0 - 6.0));

        // Assert EquityPoint on Exit is correct
        assert_eq!(
            position.meta.exit_balance.unwrap().total,
            current_balance.total + (100.0 - 200.0 - 6.0)
        )
    }

    #[test]
    fn exit_long_position_with_long_entry_fill_and_return_err() -> Result<(), String> {
        // Initial Position
        let mut position = position();
        position.direction = Direction::Short;
        position.quantity = -1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unrealised_profit_loss = position.enter_fees_total * -2.0;

        // Input Portfolio Current Balance
        let current_balance = Balance {
            timestamp: Utc::now(),
            total: 10000.0,
            available: 10000.0
        };

        // Input FillEvent
        let mut input_fill = fill_event();
        input_fill.decision = Decision::Long;
        input_fill.quantity = position.quantity;
        input_fill.fill_value_gross = 200.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };

        // Exit Position
        if let Err(_) = position.exit(current_balance, &input_fill) {
            Ok(())
        } else {
            Err(String::from(
                "Position::exit did not return an Err and it should have.",
            ))
        }
    }

    #[test]
    fn exit_short_position_with_short_entry_fill_and_return_err() -> Result<(), String> {
        // Initial Position
        let mut position = position();
        position.direction = Direction::Short;
        position.quantity = -1.0;
        position.enter_fees_total = 3.0;
        position.enter_fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };
        position.enter_avg_price_gross = 100.0;
        position.enter_value_gross = 100.0;
        position.current_symbol_price = 100.0;
        position.current_value_gross = 100.0;
        position.unrealised_profit_loss = position.enter_fees_total * -2.0;

        // Input Portfolio Current Balance
        let current_balance = Balance {
            timestamp: Utc::now(),
            total: 10000.0,
            available: 10000.0
        };

        // Input FillEvent
        let mut input_fill = fill_event();
        input_fill.decision = Decision::Short;
        input_fill.quantity = -position.quantity;
        input_fill.fill_value_gross = 200.0;
        input_fill.fees = Fees {
            exchange: 1.0,
            slippage: 1.0,
            network: 1.0,
        };

        // Exit Position
        if let Err(_) = position.exit(current_balance, &input_fill) {
            Ok(())
        } else {
            Err(String::from(
                "Position::exit did not return an Err and it should have.",
            ))
        }
    }

    #[test]
    fn calculate_avg_price_gross_correctly_with_positive_quantity() {
        let mut input_fill = fill_event();
        input_fill.fill_value_gross = 1000.0;
        input_fill.quantity = 1.0;

        let actual = Position::calculate_avg_price_gross(&input_fill);

        assert_eq!(actual, 1000.0)
    }

    #[test]
    fn calculate_avg_price_gross_correctly_with_negative_quantity() {
        let mut input_fill = fill_event();
        input_fill.fill_value_gross = 1000.0;
        input_fill.quantity = -1.0;

        let actual = Position::calculate_avg_price_gross(&input_fill);

        assert_eq!(actual, 1000.0)
    }

    #[test]
    fn parse_entry_direction_as_long_with_positive_quantity_long_decision_provided() {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::Long;
        input_fill.quantity = 1.0;

        let actual = Position::parse_entry_direction(&input_fill).unwrap();

        assert_eq!(actual, Direction::Long)
    }

    #[test]
    fn parse_entry_direction_as_short_with_negative_quantity_short_decision_provided() {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::Short;
        input_fill.quantity = -1.0;

        let actual = Position::parse_entry_direction(&input_fill).unwrap();

        assert_eq!(actual, Direction::Short)
    }

    #[test]
    fn parse_entry_direction_and_return_err_with_close_long_decision_provided() -> Result<(), String>
    {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::CloseLong;
        input_fill.quantity = -1.0;

        if let Err(_) = Position::parse_entry_direction(&input_fill) {
            Ok(())
        } else {
            Err(String::from(
                "parse_entry_direction() did not return an Err & it should.",
            ))
        }
    }

    #[test]
    fn parse_entry_direction_and_return_err_with_close_short_decision_provided(
    ) -> Result<(), String> {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::CloseShort;
        input_fill.quantity = 1.0;

        if let Err(_) = Position::parse_entry_direction(&input_fill) {
            Ok(())
        } else {
            Err(String::from(
                "parse_entry_direction() did not return an Err & it should.",
            ))
        }
    }

    #[test]
    fn parse_entry_direction_and_return_err_with_negative_quantity_long_decision_provided(
    ) -> Result<(), String> {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::Long;
        input_fill.quantity = -1.0;

        if let Err(_) = Position::parse_entry_direction(&input_fill) {
            Ok(())
        } else {
            Err(String::from(
                "parse_entry_direction() did not return an Err & it should.",
            ))
        }
    }

    #[test]
    fn parse_entry_direction_and_return_err_with_positive_quantity_short_decision_provided(
    ) -> Result<(), String> {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::Short;
        input_fill.quantity = 1.0;

        if let Err(_) = Position::parse_entry_direction(&input_fill) {
            Ok(())
        } else {
            Err(String::from(
                "parse_entry_direction() did not return an Err & it should.",
            ))
        }
    }

    #[test]
    fn calculate_unreal_profit_loss() {
        let mut long_win = position(); // Expected PnL = +8.0
        long_win.direction = Direction::Long;
        long_win.enter_value_gross = 100.0;
        long_win.enter_fees_total = 1.0;
        long_win.current_value_gross = 110.0;

        let mut long_lose = position(); // Expected PnL = -12.0
        long_lose.direction = Direction::Long;
        long_lose.enter_value_gross = 100.0;
        long_lose.enter_fees_total = 1.0;
        long_lose.current_value_gross = 90.0;

        let mut short_win = position(); // Expected PnL = +8.0
        short_win.direction = Direction::Short;
        short_win.enter_value_gross = 100.0;
        short_win.enter_fees_total = 1.0;
        short_win.current_value_gross = 90.0;

        let mut short_lose = position(); // Expected PnL = -12.0
        short_lose.direction = Direction::Short;
        short_lose.enter_value_gross = 100.0;
        short_lose.enter_fees_total = 1.0;
        short_lose.current_value_gross = 110.0;

        let inputs = vec![long_win, long_lose, short_win, short_lose];

        let expected_pnl = vec![8.0, -12.0, 8.0, -12.0];

        for (position, expected) in inputs.into_iter().zip(expected_pnl.into_iter()) {
            let actual = position.calculate_unrealised_profit_loss();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn calculate_realised_profit_loss() {
        let mut long_win = position(); // Expected PnL = +18.0
        long_win.direction = Direction::Long;
        long_win.enter_value_gross = 100.0;
        long_win.enter_fees_total = 1.0;
        long_win.exit_value_gross = 120.0;
        long_win.exit_fees_total = 1.0;

        let mut long_lose = position(); // Expected PnL = -22.0
        long_lose.direction = Direction::Long;
        long_lose.enter_value_gross = 100.0;
        long_lose.enter_fees_total = 1.0;
        long_lose.exit_value_gross = 80.0;
        long_lose.exit_fees_total = 1.0;

        let mut short_win = position(); // Expected PnL = +18.0
        short_win.direction = Direction::Short;
        short_win.enter_value_gross = 100.0;
        short_win.enter_fees_total = 1.0;
        short_win.exit_value_gross = 80.0;
        short_win.exit_fees_total = 1.0;

        let mut short_lose = position(); // Expected PnL = -22.0
        short_lose.direction = Direction::Short;
        short_lose.enter_value_gross = 100.0;
        short_lose.enter_fees_total = 1.0;
        short_lose.exit_value_gross = 120.0;
        short_lose.exit_fees_total = 1.0;

        let inputs = vec![long_win, long_lose, short_win, short_lose];

        let expected_pnl = vec![18.0, -22.0, 18.0, -22.0];

        for (position, expected) in inputs.into_iter().zip(expected_pnl.into_iter()) {
            let actual = position.calculate_realised_profit_loss();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn calculate_profit_loss_return() {
        let mut long_win = position(); // Expected Return = 0.08
        long_win.direction = Direction::Long;
        long_win.enter_value_gross = 100.0;
        long_win.realised_profit_loss = 8.0;

        let mut long_lose = position(); // Expected Return = -0.12
        long_lose.direction = Direction::Long;
        long_lose.enter_value_gross = 100.0;
        long_lose.realised_profit_loss = -12.0;

        let mut short_win = position(); // Expected Return = 0.08
        short_win.direction = Direction::Short;
        short_win.enter_value_gross = 100.0;
        short_win.realised_profit_loss = 8.0;

        let mut short_lose = position(); // Expected Return = -0.12
        short_lose.direction = Direction::Short;
        short_lose.enter_value_gross = 100.0;
        short_lose.realised_profit_loss = -12.0;

        let inputs = vec![long_win, long_lose, short_win, short_lose];

        let expected_return = vec![0.08, -0.12, 0.08, -0.12];

        for (position, expected) in inputs.into_iter().zip(expected_return.into_iter()) {
            let actual = position.calculate_profit_loss_return();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn determine_exit_decision() {
        // Direction::Long -> Decision::CloseLong
        let input = Direction::Long;
        assert_eq!(input.determine_exit_decision(), Decision::CloseLong);

        // Direction::Short -> Decision::CloseShort
        let input = Direction::Short;
        assert_eq!(input.determine_exit_decision(), Decision::CloseShort);
    }

    #[test]
    fn position_update_from_position() {
        let mut input_position = position();
        input_position.current_symbol_price = 100.0;
        input_position.current_value_gross = 200.0;
        input_position.unrealised_profit_loss = 150.0;

        let actual_update = PositionUpdate::from(&mut input_position);

        assert_eq!(
            actual_update.current_symbol_price,
            input_position.current_symbol_price
        );
        assert_eq!(
            actual_update.current_value_gross,
            input_position.current_value_gross
        );
        assert_eq!(
            actual_update.unrealised_profit_loss,
            input_position.unrealised_profit_loss
        );
    }

    #[test]
    fn position_exit_try_from_exited_position() {
        let timestamp = Utc::now();

        let mut exited_position = position();
        exited_position.meta.last_update_trace_id = Uuid::new_v4();
        exited_position.meta.last_update_timestamp = timestamp;
        exited_position.meta.exit_balance = Some(Balance {
            timestamp,
            total: 0.0,
            available: 0.0
        });

        exited_position.exit_fees = Fees {
            exchange: 0.0,
            slippage: 0.0,
            network: 0.0,
        };
        exited_position.exit_fees_total = 0.0;
        exited_position.exit_avg_price_gross = 100.0;
        exited_position.exit_value_gross = 100.0;
        exited_position.realised_profit_loss = 100.0;

        let actual_exit = PositionExit::try_from(&mut exited_position).unwrap();

        assert_eq!(
            actual_exit.exit_balance,
            exited_position.meta.exit_balance.unwrap()
        );
        assert_eq!(actual_exit.exit_fees, exited_position.exit_fees);
        assert_eq!(actual_exit.exit_fees_total, exited_position.exit_fees_total);
        assert_eq!(
            actual_exit.exit_avg_price_gross,
            exited_position.exit_avg_price_gross
        );
        assert_eq!(
            actual_exit.exit_value_gross,
            exited_position.exit_value_gross
        );
        assert_eq!(
            actual_exit.realised_profit_loss,
            exited_position.realised_profit_loss
        );
    }

    #[test]
    fn position_exit_try_from_open_position() {
        let mut exited_position = position();
        exited_position.meta.exit_balance = None;

        assert!(PositionExit::try_from(&mut exited_position).is_err());
    }
}