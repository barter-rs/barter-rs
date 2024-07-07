use crate::{
    execution::{FeeAmount, Fees, FillEvent},
    portfolio::{error::PortfolioError, Balance},
    strategy::Decision,
};
use barter_data::event::{DataKind, MarketEvent};
use barter_integration::model::{instrument::Instrument, Exchange, Side};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use uuid::Uuid;

/// Enters a new [`Position`].
pub trait PositionEnterer {
    /// Returns a new [`Position`], given an input [`FillEvent`] & an associated engine_id.
    fn enter(engine_id: Uuid, fill: &FillEvent) -> Result<Position, PortfolioError>;
}

/// Updates an open [`Position`].
pub trait PositionUpdater {
    /// Updates an open [`Position`] using the latest input [`MarketEvent`], returning a
    /// [`PositionUpdate`] that communicates the open [`Position`]'s change in state.
    fn update(&mut self, market: &MarketEvent<DataKind>) -> Option<PositionUpdate>;
}

/// Exits an open [`Position`].
pub trait PositionExiter {
    /// Exits an open [`Position`], given the input Portfolio equity & the [`FillEvent`] returned
    /// from an Execution handler.
    fn exit(&mut self, balance: Balance, fill: &FillEvent) -> Result<PositionExit, PortfolioError>;
}

/// Communicates a String represents a unique [`Position`] identifier.
pub type PositionId = String;

/// Returns a unique identifier for a [`Position`] given an engine_id, [`Exchange`] & [`Instrument`].
pub fn determine_position_id(
    engine_id: Uuid,
    exchange: &Exchange,
    instrument: &Instrument,
) -> PositionId {
    format!("{}_{}_{}_position", engine_id, exchange, instrument)
}

/// Data encapsulating the state of an ongoing or closed [`Position`].
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Position {
    /// Unique identifier for a [`Position`] generated from an engine_id, [`Exchange`] & [`Instrument`].
    pub position_id: PositionId,

    /// Metadata detailing trace UUIDs, timestamps & equity associated with entering, updating & exiting.
    pub meta: PositionMeta,

    /// [`Exchange`] associated with this [`Position`].
    pub exchange: Exchange,

    /// [`Instrument`] associated with this [`Position`].
    pub instrument: Instrument,

    /// Buy or Sell.
    ///
    /// Notes:
    /// - Side::Buy considered synonymous with Long.
    /// - Side::Sell considered synonymous with Short.
    pub side: Side,

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
            enter_time: fill.market_meta.time,
            update_time: fill.time,
            exit_balance: None,
        };

        // Enter fees
        let enter_fees_total = fill.fees.calculate_total_fees();

        // Enter price
        let enter_avg_price_gross = Position::calculate_avg_price_gross(fill);

        // Unreal profit & loss
        let unrealised_profit_loss = -enter_fees_total * 2.0;

        Ok(Position {
            position_id: determine_position_id(engine_id, &fill.exchange, &fill.instrument),
            exchange: fill.exchange.clone(),
            instrument: fill.instrument.clone(),
            meta: metadata,
            side: Position::parse_entry_side(fill)?,
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
    fn update(&mut self, market: &MarketEvent<DataKind>) -> Option<PositionUpdate> {
        // Determine close from MarketEvent
        let close = match &market.kind {
            DataKind::Trade(trade) => trade.price,
            DataKind::Candle(candle) => candle.close,
            DataKind::OrderBookL1(book_l1) => book_l1.volume_weighed_mid_price(),
            DataKind::OrderBook(book) => book.volume_weighed_mid_price()?,
            DataKind::Liquidation(_) => return None,
        };

        self.meta.update_time = market.exchange_time;

        self.current_symbol_price = close;

        // Market value gross
        self.current_value_gross = close * self.quantity.abs();

        // Unreal profit & loss
        self.unrealised_profit_loss = self.calculate_unrealised_profit_loss();

        // Return a PositionUpdate event that communicates the change in state
        Some(PositionUpdate::from(self))
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
        self.meta.update_time = fill.time;
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

    /// Determine the [`Position`] entry [`Side`] by analysing the input [`FillEvent`].
    pub fn parse_entry_side(fill: &FillEvent) -> Result<Side, PortfolioError> {
        match fill.decision {
            Decision::Long if fill.quantity.is_sign_positive() => Ok(Side::Buy),
            Decision::Short if fill.quantity.is_sign_negative() => Ok(Side::Sell),
            Decision::CloseLong | Decision::CloseShort => {
                Err(PortfolioError::CannotEnterPositionWithExitFill)
            }
            _ => Err(PortfolioError::ParseEntrySide),
        }
    }

    /// Determines the [`Decision`] required to exit this [`Side`] (Buy or Sell) [`Position`].
    pub fn determine_exit_decision(&self) -> Decision {
        match self.side {
            Side::Buy => Decision::CloseLong,
            Side::Sell => Decision::CloseShort,
        }
    }

    /// Calculate the approximate [`Position::unrealised_profit_loss`] of a [`Position`].
    pub fn calculate_unrealised_profit_loss(&self) -> f64 {
        let approx_total_fees = self.enter_fees_total * 2.0;

        match self.side {
            Side::Buy => self.current_value_gross - self.enter_value_gross - approx_total_fees,
            Side::Sell => self.enter_value_gross - self.current_value_gross - approx_total_fees,
        }
    }

    /// Calculate the exact [`Position::realised_profit_loss`] of a [`Position`].
    pub fn calculate_realised_profit_loss(&self) -> f64 {
        let total_fees = self.enter_fees_total + self.exit_fees_total;

        match self.side {
            Side::Buy => self.exit_value_gross - self.enter_value_gross - total_fees,
            Side::Sell => self.enter_value_gross - self.exit_value_gross - total_fees,
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
    pub exchange: Option<Exchange>,
    pub instrument: Option<Instrument>,
    pub meta: Option<PositionMeta>,
    pub side: Option<Side>,
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

    pub fn exchange(self, value: Exchange) -> Self {
        Self {
            exchange: Some(value),
            ..self
        }
    }

    pub fn instrument(self, value: Instrument) -> Self {
        Self {
            instrument: Some(value),
            ..self
        }
    }

    pub fn meta(self, value: PositionMeta) -> Self {
        Self {
            meta: Some(value),
            ..self
        }
    }

    pub fn side(self, value: Side) -> Self {
        Self {
            side: Some(value),
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
        Ok(Position {
            position_id: self
                .position_id
                .ok_or(PortfolioError::BuilderIncomplete("position_id"))?,
            exchange: self
                .exchange
                .ok_or(PortfolioError::BuilderIncomplete("exchange"))?,
            instrument: self
                .instrument
                .ok_or(PortfolioError::BuilderIncomplete("instrument"))?,
            meta: self.meta.ok_or(PortfolioError::BuilderIncomplete("meta"))?,
            side: self.side.ok_or(PortfolioError::BuilderIncomplete("side"))?,
            quantity: self
                .quantity
                .ok_or(PortfolioError::BuilderIncomplete("quantity"))?,
            enter_fees: self
                .enter_fees
                .ok_or(PortfolioError::BuilderIncomplete("enter_fees"))?,
            enter_fees_total: self
                .enter_fees_total
                .ok_or(PortfolioError::BuilderIncomplete("enter_fees_total"))?,
            enter_avg_price_gross: self
                .enter_avg_price_gross
                .ok_or(PortfolioError::BuilderIncomplete("enter_avg_price_gross"))?,
            enter_value_gross: self
                .enter_value_gross
                .ok_or(PortfolioError::BuilderIncomplete("enter_value_gross"))?,
            exit_fees: self
                .exit_fees
                .ok_or(PortfolioError::BuilderIncomplete("exit_fees"))?,
            exit_fees_total: self
                .exit_fees_total
                .ok_or(PortfolioError::BuilderIncomplete("exit_fees_total"))?,
            exit_avg_price_gross: self
                .exit_avg_price_gross
                .ok_or(PortfolioError::BuilderIncomplete("exit_avg_price_gross"))?,
            exit_value_gross: self
                .exit_value_gross
                .ok_or(PortfolioError::BuilderIncomplete("exit_value_gross"))?,
            current_symbol_price: self
                .current_symbol_price
                .ok_or(PortfolioError::BuilderIncomplete("current_symbol_price"))?,
            current_value_gross: self
                .current_value_gross
                .ok_or(PortfolioError::BuilderIncomplete("current_value_gross"))?,
            unrealised_profit_loss: self
                .unrealised_profit_loss
                .ok_or(PortfolioError::BuilderIncomplete("unrealised_profit_loss"))?,
            realised_profit_loss: self
                .realised_profit_loss
                .ok_or(PortfolioError::BuilderIncomplete("realised_profit_loss"))?,
        })
    }
}

/// Metadata detailing the trace UUIDs & timestamps associated with entering, updating & exiting
/// a [`Position`].
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PositionMeta {
    /// [`FillEvent`] timestamp that triggered the entering of this [`Position`].
    pub enter_time: DateTime<Utc>,

    /// Timestamp of the last event to trigger a [`Position`] state change (enter, update, exit).
    pub update_time: DateTime<Utc>,

    /// Portfolio [`Balance`] calculated at the point of exiting a [`Position`].
    pub exit_balance: Option<Balance>,
}

impl Default for PositionMeta {
    fn default() -> Self {
        Self {
            enter_time: Utc::now(),
            update_time: Utc::now(),
            exit_balance: None,
        }
    }
}

/// [`Position`] update event. Occurs as a result of receiving new [`MarketEvent`] data.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PositionUpdate {
    /// Unique identifier for a [`Position`], generated from an exchange, symbol, and enter_time.
    pub position_id: String,
    /// Event timestamp of the last event to trigger a [`Position`] update.
    pub update_time: DateTime<Utc>,
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
            update_time: updated_position.meta.update_time,
            current_symbol_price: updated_position.current_symbol_price,
            current_value_gross: updated_position.current_value_gross,
            unrealised_profit_loss: updated_position.unrealised_profit_loss,
        }
    }
}

/// [`Position`] exit event. Occurs as a result of a [`FillEvent`] that exits a [`Position`].
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PositionExit {
    /// Unique identifier for a [`Position`], generated from an exchange, symbol, and enter_time.
    pub position_id: String,

    /// [`FillEvent`] timestamp that triggered the exiting of this [`Position`].
    pub exit_time: DateTime<Utc>,

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
            exit_time: exited_position.meta.update_time,
            exit_balance: exited_position
                .meta
                .exit_balance
                .ok_or(PortfolioError::PositionExit)?,
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
    use crate::test_util::{fill_event, market_event_trade, position};
    use barter_integration::model::Side;

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

        assert_eq!(position.side, Side::Buy);
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

        assert_eq!(position.side, Side::Sell);
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
        position.side = Side::Buy;
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
        let mut input_market = market_event_trade(Side::Buy);
        match input_market.kind {
            // +100.0 higher than current_symbol_price
            DataKind::Candle(ref mut candle) => candle.close = 200.0,
            DataKind::Trade(ref mut trade) => trade.price = 200.0,
            _ => todo!(),
        };

        // Update Position
        position.update(&input_market);

        // Assert update hasn't changed fields that are constant after creation
        assert_eq!(position.side, Side::Buy);
        assert_eq!(position.quantity, 1.0);
        assert_eq!(position.enter_fees_total, 3.0);
        assert_eq!(position.enter_fees.exchange, 1.0);
        assert_eq!(position.enter_fees.slippage, 1.0);
        assert_eq!(position.enter_fees.network, 1.0);
        assert_eq!(position.enter_avg_price_gross, 100.0);
        assert_eq!(position.enter_value_gross, 100.0);

        // Assert updated fields are correct
        let close = match &input_market.kind {
            DataKind::Trade(trade) => trade.price,
            DataKind::Candle(candle) => candle.close,
            _ => todo!(),
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
        position.side = Side::Buy;
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
        let mut input_market = market_event_trade(Side::Sell);

        match input_market.kind {
            // -50.0 lower than current_symbol_price
            DataKind::Candle(ref mut candle) => candle.close = 50.0,
            DataKind::Trade(ref mut trade) => trade.price = 50.0,
            _ => todo!(),
        };

        // Update Position
        position.update(&input_market);

        // Assert update hasn't changed fields that are constant after creation
        assert_eq!(position.side, Side::Buy);
        assert_eq!(position.quantity, 1.0);
        assert_eq!(position.enter_fees_total, 3.0);
        assert_eq!(position.enter_fees.exchange, 1.0);
        assert_eq!(position.enter_fees.slippage, 1.0);
        assert_eq!(position.enter_fees.network, 1.0);
        assert_eq!(position.enter_avg_price_gross, 100.0);
        assert_eq!(position.enter_value_gross, 100.0);

        // Assert updated fields are correct
        let close = match &input_market.kind {
            DataKind::Trade(trade) => trade.price,
            DataKind::Candle(candle) => candle.close,
            _ => todo!(),
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
        position.side = Side::Sell;
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
        let mut input_market = market_event_trade(Side::Buy);

        match input_market.kind {
            // -50.0 lower than current_symbol_price
            DataKind::Candle(ref mut candle) => candle.close = 50.0,
            DataKind::Trade(ref mut trade) => trade.price = 50.0,
            _ => todo!(),
        };

        // Update Position
        position.update(&input_market);

        // Assert update hasn't changed fields that are constant after creation
        assert_eq!(position.side, Side::Sell);
        assert_eq!(position.quantity, -1.0);
        assert_eq!(position.enter_fees_total, 3.0);
        assert_eq!(position.enter_fees.exchange, 1.0);
        assert_eq!(position.enter_fees.slippage, 1.0);
        assert_eq!(position.enter_fees.network, 1.0);
        assert_eq!(position.enter_avg_price_gross, 100.0);
        assert_eq!(position.enter_value_gross, 100.0);

        // Assert updated fields are correct
        let close = match &input_market.kind {
            DataKind::Trade(trade) => trade.price,
            DataKind::Candle(candle) => candle.close,
            _ => todo!(),
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
        position.side = Side::Sell;
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
        let mut input_market = market_event_trade(Side::Sell);

        match input_market.kind {
            // +100.0 higher than current_symbol_price
            DataKind::Candle(ref mut candle) => candle.close = 200.0,
            DataKind::Trade(ref mut trade) => trade.price = 200.0,
            _ => todo!(),
        };

        // Update Position
        position.update(&input_market);

        // Assert update hasn't changed fields that are constant after creation
        assert_eq!(position.side, Side::Sell);
        assert_eq!(position.quantity, -1.0);
        assert_eq!(position.enter_fees_total, 3.0);
        assert_eq!(position.enter_fees.exchange, 1.0);
        assert_eq!(position.enter_fees.slippage, 1.0);
        assert_eq!(position.enter_fees.network, 1.0);
        assert_eq!(position.enter_avg_price_gross, 100.0);
        assert_eq!(position.enter_value_gross, 100.0);

        // Assert updated fields are correct
        let close = match &input_market.kind {
            DataKind::Trade(trade) => trade.price,
            DataKind::Candle(candle) => candle.close,
            _ => todo!(),
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
        position.side = Side::Buy;
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
            time: Utc::now(),
            total: 10000.0,
            available: 10000.0,
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
        assert_eq!(position.side, Side::Buy);
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
        position.side = Side::Buy;
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
            time: Utc::now(),
            total: 10000.0,
            available: 10000.0,
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
        assert_eq!(position.side, Side::Buy);
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
        position.side = Side::Sell;
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
            time: Utc::now(),
            total: 10000.0,
            available: 10000.0,
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
        assert_eq!(position.side, Side::Sell);
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
        position.side = Side::Sell;
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
            time: Utc::now(),
            total: 10000.0,
            available: 10000.0,
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
        assert_eq!(position.side, Side::Sell);
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
        position.side = Side::Sell;
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
            time: Utc::now(),
            total: 10000.0,
            available: 10000.0,
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
        position.side = Side::Sell;
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
            time: Utc::now(),
            total: 10000.0,
            available: 10000.0,
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
    fn parse_entry_side_as_long_with_positive_quantity_long_decision_provided() {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::Long;
        input_fill.quantity = 1.0;

        let actual = Position::parse_entry_side(&input_fill).unwrap();

        assert_eq!(actual, Side::Buy)
    }

    #[test]
    fn parse_entry_side_as_short_with_negative_quantity_short_decision_provided() {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::Short;
        input_fill.quantity = -1.0;

        let actual = Position::parse_entry_side(&input_fill).unwrap();

        assert_eq!(actual, Side::Sell)
    }

    #[test]
    fn parse_entry_side_and_return_err_with_close_long_decision_provided() -> Result<(), String> {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::CloseLong;
        input_fill.quantity = -1.0;

        if let Err(_) = Position::parse_entry_side(&input_fill) {
            Ok(())
        } else {
            Err(String::from(
                "parse_entry_side() did not return an Err & it should.",
            ))
        }
    }

    #[test]
    fn parse_entry_side_and_return_err_with_close_short_decision_provided() -> Result<(), String> {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::CloseShort;
        input_fill.quantity = 1.0;

        if let Err(_) = Position::parse_entry_side(&input_fill) {
            Ok(())
        } else {
            Err(String::from(
                "parse_entry_side() did not return an Err & it should.",
            ))
        }
    }

    #[test]
    fn parse_entry_side_and_return_err_with_negative_quantity_long_decision_provided(
    ) -> Result<(), String> {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::Long;
        input_fill.quantity = -1.0;

        if let Err(_) = Position::parse_entry_side(&input_fill) {
            Ok(())
        } else {
            Err(String::from(
                "parse_entry_side() did not return an Err & it should.",
            ))
        }
    }

    #[test]
    fn parse_entry_side_and_return_err_with_positive_quantity_short_decision_provided(
    ) -> Result<(), String> {
        let mut input_fill = fill_event();
        input_fill.decision = Decision::Short;
        input_fill.quantity = 1.0;

        if let Err(_) = Position::parse_entry_side(&input_fill) {
            Ok(())
        } else {
            Err(String::from(
                "parse_entry_side() did not return an Err & it should.",
            ))
        }
    }

    #[test]
    fn calculate_unreal_profit_loss() {
        let mut long_win = position(); // Expected PnL = +8.0
        long_win.side = Side::Buy;
        long_win.enter_value_gross = 100.0;
        long_win.enter_fees_total = 1.0;
        long_win.current_value_gross = 110.0;

        let mut long_lose = position(); // Expected PnL = -12.0
        long_lose.side = Side::Buy;
        long_lose.enter_value_gross = 100.0;
        long_lose.enter_fees_total = 1.0;
        long_lose.current_value_gross = 90.0;

        let mut short_win = position(); // Expected PnL = +8.0
        short_win.side = Side::Sell;
        short_win.enter_value_gross = 100.0;
        short_win.enter_fees_total = 1.0;
        short_win.current_value_gross = 90.0;

        let mut short_lose = position(); // Expected PnL = -12.0
        short_lose.side = Side::Sell;
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
        long_win.side = Side::Buy;
        long_win.enter_value_gross = 100.0;
        long_win.enter_fees_total = 1.0;
        long_win.exit_value_gross = 120.0;
        long_win.exit_fees_total = 1.0;

        let mut long_lose = position(); // Expected PnL = -22.0
        long_lose.side = Side::Buy;
        long_lose.enter_value_gross = 100.0;
        long_lose.enter_fees_total = 1.0;
        long_lose.exit_value_gross = 80.0;
        long_lose.exit_fees_total = 1.0;

        let mut short_win = position(); // Expected PnL = +18.0
        short_win.side = Side::Sell;
        short_win.enter_value_gross = 100.0;
        short_win.enter_fees_total = 1.0;
        short_win.exit_value_gross = 80.0;
        short_win.exit_fees_total = 1.0;

        let mut short_lose = position(); // Expected PnL = -22.0
        short_lose.side = Side::Sell;
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
        long_win.side = Side::Buy;
        long_win.enter_value_gross = 100.0;
        long_win.realised_profit_loss = 8.0;

        let mut long_lose = position(); // Expected Return = -0.12
        long_lose.side = Side::Buy;
        long_lose.enter_value_gross = 100.0;
        long_lose.realised_profit_loss = -12.0;

        let mut short_win = position(); // Expected Return = 0.08
        short_win.side = Side::Sell;
        short_win.enter_value_gross = 100.0;
        short_win.realised_profit_loss = 8.0;

        let mut short_lose = position(); // Expected Return = -0.12
        short_lose.side = Side::Sell;
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
    fn position_determine_exit_decision() {
        // Side::Buy -> Decision::CloseLong
        let mut position = position();
        position.side = Side::Buy;
        assert_eq!(position.determine_exit_decision(), Decision::CloseLong);

        // Side::Buy -> Decision::CloseShort
        position.side = Side::Sell;
        assert_eq!(position.determine_exit_decision(), Decision::CloseShort);
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
        let time = Utc::now();

        let mut exited_position = position();
        exited_position.meta.update_time = time;
        exited_position.meta.exit_balance = Some(Balance {
            time,
            total: 0.0,
            available: 0.0,
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
