use barter_execution::trade::{AssetFees, Trade, TradeId};
use barter_instrument::{
    Side,
    asset::{AssetIndex, QuoteAsset},
    instrument::InstrumentIndex,
};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::error;

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct PositionManager<InstrumentKey = InstrumentIndex> {
    pub current: Option<Position<QuoteAsset, InstrumentKey>>,
}

impl<InstrumentKey> Default for PositionManager<InstrumentKey> {
    fn default() -> Self {
        Self { current: None }
    }
}

impl<InstrumentKey> PositionManager<InstrumentKey> {
    /// Updates the current position state based on a new trade.
    ///
    /// This method handles:
    /// - Opening a new position if none exists
    /// - Updating an existing position (increase/decrease/close)
    /// - Handling position flips (close existing & open new with any remaining trade quantity)
    pub fn update_from_trade(
        &mut self,
        trade: &Trade<QuoteAsset, InstrumentKey>,
    ) -> Option<PositionExited<QuoteAsset, InstrumentKey>>
    where
        InstrumentKey: Debug + Clone + PartialEq,
    {
        let (current, closed) = match self.current.take() {
            Some(position) => {
                // Update current Position, maybe closing it, and maybe opening a new Position
                // with leftover trade.quantity
                position.update_from_trade(trade)
            }
            None => {
                // No current Position, so enter a new one with Trade
                (Some(Position::from(trade)), None)
            }
        };

        self.current = current;

        closed
    }
}

/// Represents an open trading position for a specific instrument.
///
/// # Type Parameters
/// - `AssetKey`: The type representing the asset used for fees (e.g. AssetIndex, QuoteAsset, etc.)
/// - `InstrumentKey`: The type identifying the traded instrument (e.g. InstrumentIndex, etc.)
///
/// # Examples
/// ## Partially Reduce LONG Position
/// ```rust
/// use barter::engine::state::position::Position;
/// use barter_execution::order::id::{OrderId, StrategyId};
/// use barter_execution::trade::{AssetFees, Trade, TradeId};
/// use barter_instrument::asset::QuoteAsset;
/// use barter_instrument::instrument::name::InstrumentNameInternal;
/// use barter_instrument::Side;
/// use chrono::{DateTime, Utc};
/// use std::str::FromStr;
/// use rust_decimal_macros::dec;
///
/// // Create a new LONG Position from an initial Buy trade
/// let position = Position::from(&Trade {
///     id: TradeId::new("trade_1"),
///     order_id: OrderId::new("order_1"),
///     instrument: InstrumentNameInternal::new("BTC-USD"),
///     strategy: StrategyId::new("strategy_1"),
///     time_exchange: DateTime::from_str("2024-01-01T00:00:00Z").unwrap(),
///     side: Side::Buy,
///     price: dec!(50_000.0),
///     quantity: dec!(0.1),
///     fees: AssetFees::quote_fees(dec!(5.0))
/// });
/// assert_eq!(position.side, Side::Buy);
/// assert_eq!(position.quantity_abs, dec!(0.1));
///
/// // Partially reduce LONG Position from a new Sell Trade
/// let (updated_position, closed_position) = position.update_from_trade(&Trade {
///     id: TradeId::new("trade_2"),
///     order_id: OrderId::new("order_2"),
///     instrument: InstrumentNameInternal::new("BTC-USD"),
///     strategy: StrategyId::new("strategy_1"),
///     time_exchange: DateTime::from_str("2024-01-01T01:00:00Z").unwrap(),
///     side: Side::Sell,
///     price: dec!(60_000.0),
///     quantity: dec!(0.05),
///     fees: AssetFees::quote_fees(dec!(2.5))
/// });
///
/// // LONG Position is still open, but with reduced size
/// let updated_position = updated_position.unwrap();
/// assert_eq!(updated_position.quantity_abs, dec!(0.05));
/// assert_eq!(updated_position.quantity_abs_max, dec!(0.1));
/// assert_eq!(updated_position.pnl_realised, dec!(492.5));
/// assert!(closed_position.is_none());
/// ```
///
/// ## Flip Position - Close SHORT and Open LONG
/// ```rust
/// use barter::engine::state::position::Position;
/// use barter_execution::order::id::{OrderId, StrategyId};
/// use barter_execution::trade::{AssetFees, Trade, TradeId};
/// use barter_instrument::asset::QuoteAsset;
/// use barter_instrument::instrument::name::InstrumentNameInternal;
/// use barter_instrument::Side;
/// use chrono::{DateTime, Utc};
/// use std::str::FromStr;
/// use rust_decimal_macros::dec;
///
/// // Create a new SHORT Position from an initial Sell trade
/// let position = Position::from(&Trade {
///     id: TradeId::new("trade_1"),
///     order_id: OrderId::new("order_1"),
///     instrument: InstrumentNameInternal::new("BTC-USD"),
///     strategy: StrategyId::new("strategy_1"),
///     time_exchange: DateTime::from_str("2024-01-01T00:00:00Z").unwrap(),
///     side: Side::Sell,
///     price: dec!(50_000.0),
///     quantity: dec!(0.1),
///     fees: AssetFees::quote_fees(dec!(5.0))
/// });
/// assert_eq!(position.side, Side::Sell);
/// assert_eq!(position.quantity_abs, dec!(0.1));
///
/// // Close SHORT from a new Buy trade with larger quantity, flipping into a new LONG Position
/// let (new_position, closed_position) = position.update_from_trade(&Trade {
///     id: TradeId::new("trade_2"),
///     order_id: OrderId::new("order_2"),
///     instrument: InstrumentNameInternal::new("BTC-USD"),
///     strategy: StrategyId::new("strategy_1"),
///     time_exchange: DateTime::from_str("2024-01-01T01:00:00Z").unwrap(),
///     side: Side::Buy,
///     price: dec!(40_000.0),
///     quantity: dec!(0.2),
///     fees: AssetFees::quote_fees(dec!(10.0))
/// });
///
/// // Original SHORT Position closed with profit
/// let closed = closed_position.unwrap();
/// assert_eq!(closed.side, Side::Sell);
/// assert_eq!(closed.quantity_abs_max, dec!(0.1));
/// assert_eq!(closed.pnl_realised, dec!(990.0));
///
/// // New LONG Position opened with remaining quantity & proportional fees
/// let new_position = new_position.unwrap();
/// assert_eq!(new_position.side, Side::Buy);
/// assert_eq!(new_position.quantity_abs, dec!(0.1));
/// assert_eq!(new_position.price_entry_average, dec!(40_000.0));
/// assert_eq!(new_position.pnl_realised, dec!(-5.0));
/// ```
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct Position<AssetKey = AssetIndex, InstrumentKey = InstrumentIndex> {
    /// [`Position`] Instrument identifier (eg/ InstrumentIndex, InstrumentNameInternal, etc.).
    pub instrument: InstrumentKey,

    /// [`Position`] direction (Side::Buy => LONG, Side::Sell => SHORT).
    pub side: Side,

    /// Volume-weighted average entry price across all [`Position`] increasing [`Trade`]s.
    pub price_entry_average: Decimal,

    /// Current absolute [`Position`] quantity.
    pub quantity_abs: Decimal,

    /// Maximum absolute [`Position`] quantity reached by all entry/increase [`Trade`]s.
    pub quantity_abs_max: Decimal,

    /// Estimated unrealised PnL generated from closing the remaining [`Position`] `quantity_abs`.
    ///
    /// Note this includes estimated exit fees.
    pub pnl_unrealised: Decimal,

    /// Cumulative realised PnL from any partially closed [`Position`] `quantity_abs_max`.
    ///
    /// Note this includes fees.
    pub pnl_realised: Decimal,

    /// Cumulative fees paid when entering/increasing [`Position`] quantity.
    pub fees_enter: AssetFees<AssetKey>,

    /// Cumulative fees paid when exiting/reducing [`Position`] quantity.
    pub fees_exit: AssetFees<AssetKey>,

    /// Timestamp of [`Trade`] that triggered the initial [`Position`] entry.
    pub time_enter: DateTime<Utc>,

    /// Timestamp of most recent [`Position`] update.
    ///
    /// Note this could be an update triggered by a [`Trade`], or a `pnl_unrealised` update by a
    /// new market price.
    pub time_exchange_update: DateTime<Utc>,

    /// [`TradeId`]s of all the [`Trade`]s associated with this [`Position`].
    pub trades: Vec<TradeId>,
}

impl<InstrumentKey> Position<QuoteAsset, InstrumentKey> {
    /// Updates the [`Position`] state based on a new [`Trade`].
    ///
    /// This method handles various scenarios:
    /// - Increasing an existing [`Position`] (same [`Side`] [`Trade`]).
    /// - Reducing an existing [`Position`] (opposite [`Side`], partially closing some quantity).
    /// - Closing a [`Position`] exactly (opposite [`Side`], fully closing quantity).
    /// - Flipping a [`Position`] - closing and opening a new [`Position`] on the opposite [`Side`].
    ///
    /// # Arguments
    /// * `trade` - The new trade to process
    ///
    /// # Returns
    /// A tuple containing:
    /// - `Option<Position>`: The updated [`Position`], unless it was exactly closed.
    /// - `Option<PositionExited>`: The closed [`PositionExited`], if the [`Position`] was closed.
    pub fn update_from_trade(
        mut self,
        trade: &Trade<QuoteAsset, InstrumentKey>,
    ) -> (
        Option<Self>,
        Option<PositionExited<QuoteAsset, InstrumentKey>>,
    )
    where
        InstrumentKey: Debug + Clone + PartialEq,
    {
        // Sanity check
        if self.instrument != trade.instrument {
            error!(
                position = ?self,
                trade = ?trade,
                "Position tried to be updated from a Trade for a different Instrument - ignoring"
            );
            return (Some(self), None);
        }

        // Add TradeId to current Position
        self.trades.push(trade.id.clone());

        use Side::*;
        match (self.side, trade.side) {
            // Increase LONG/SHORT Position
            (Buy, Buy) | (Sell, Sell) => {
                self.update_price_entry_average(trade);
                self.quantity_abs += trade.quantity.abs();
                if self.quantity_abs > self.quantity_abs_max {
                    self.quantity_abs_max = self.quantity_abs;
                }
                self.pnl_realised -= trade.fees.fees;
                self.fees_enter.fees += trade.fees.fees;
                self.time_exchange_update = trade.time_exchange;
                self.update_pnl_unrealised(trade.price);

                (Some(self), None)
            }
            // Reduce LONG/SHORT Position
            (Buy, Sell) | (Sell, Buy) if self.quantity_abs > trade.quantity.abs() => {
                // Update pnl_realised
                self.update_pnl_realised(trade.quantity, trade.price, trade.fees.fees);

                // Update remaining Position state
                self.quantity_abs -= trade.quantity.abs();
                self.fees_exit.fees += trade.fees.fees;
                self.time_exchange_update = trade.time_exchange;

                // Update pnl_unrealised for remaining Position
                self.update_pnl_unrealised(trade.price);

                (Some(self), None)
            }
            // Close LONG/SHORT Position (exactly)
            (Buy, Sell) | (Sell, Buy) if self.quantity_abs == trade.quantity.abs() => {
                self.quantity_abs -= trade.quantity.abs();
                self.fees_exit.fees += trade.fees.fees;
                self.time_exchange_update = trade.time_exchange;
                self.update_pnl_realised(trade.quantity, trade.price, trade.fees.fees);
                self.update_pnl_unrealised(trade.price);

                (None, Some(PositionExited::from(self)))
            }

            // Close LONG/SHORT Position & open SHORT/LONG with remaining trade.quantity
            (Buy, Sell) | (Sell, Buy) if self.quantity_abs < trade.quantity.abs() => {
                // Trade flips Position, so generate theoretical initial Trade for next Position
                let next_position_quantity = trade.quantity.abs() - self.quantity_abs;
                let next_position_fee_enter =
                    trade.fees.fees * (next_position_quantity / trade.quantity.abs());
                let next_position_trade = Trade {
                    id: trade.id.clone(),
                    order_id: trade.order_id.clone(),
                    instrument: trade.instrument.clone(),
                    strategy: trade.strategy.clone(),
                    time_exchange: trade.time_exchange,
                    side: trade.side,
                    price: trade.price,
                    quantity: next_position_quantity,
                    fees: AssetFees {
                        asset: trade.fees.asset.clone(),
                        fees: next_position_fee_enter,
                    },
                };

                // Update closing Position with appropriate ratio of fees for theoretical quantity
                let fee_exit = trade.fees.fees * (self.quantity_abs / trade.quantity.abs());
                self.fees_exit.fees += fee_exit;
                self.time_exchange_update = trade.time_exchange;
                self.update_pnl_realised(self.quantity_abs, trade.price, fee_exit);
                self.quantity_abs = Decimal::ZERO;
                self.update_pnl_unrealised(trade.price);

                (
                    Some(Self::from(&next_position_trade)),
                    Some(PositionExited::from(self)),
                )
            }
            _ => unreachable!("match expression guard statements cover all cases"),
        }
    }

    /// Updates the volume-weighted average entry price of the [`Position`].
    ///
    /// Internally uses the logic defined in [`calculate_price_entry_average`].
    fn update_price_entry_average(&mut self, trade: &Trade<QuoteAsset, InstrumentKey>) {
        self.price_entry_average = calculate_price_entry_average(
            self.price_entry_average,
            self.quantity_abs,
            trade.price,
            trade.quantity.abs(),
        );
    }

    /// Update [`Position::pnl_unrealised`](Position) with the estimated PnL from closing
    /// the [`Position`] at the provided price.
    ///
    /// Note that this could be called with a recent [`Trade`] price, or a price generated from
    /// a model based on public market data.
    pub fn update_pnl_unrealised(&mut self, price: Decimal) {
        self.pnl_unrealised = calculate_pnl_unrealised(
            self.side,
            self.price_entry_average,
            self.quantity_abs,
            self.quantity_abs_max,
            self.fees_enter.fees,
            price,
        );
    }

    /// Updates the [`Position`] `pnl_realised` from a closed portion of the [`Position`] quantity.
    pub fn update_pnl_realised(
        &mut self,
        closed_quantity: Decimal,
        closed_price: Decimal,
        closed_fee: Decimal,
    ) {
        // Update total Position pnl_realised with closed quantity PnL
        self.pnl_realised += calculate_pnl_realised(
            self.side,
            self.price_entry_average,
            closed_quantity,
            closed_price,
            closed_fee,
        );
    }
}

impl<InstrumentKey> From<&Trade<QuoteAsset, InstrumentKey>> for Position<QuoteAsset, InstrumentKey>
where
    InstrumentKey: Clone,
{
    fn from(trade: &Trade<QuoteAsset, InstrumentKey>) -> Self {
        let mut trades = Vec::with_capacity(2);
        trades.push(trade.id.clone());
        Self {
            instrument: trade.instrument.clone(),
            side: trade.side,
            price_entry_average: trade.price,
            quantity_abs: trade.quantity.abs(),
            quantity_abs_max: trade.quantity.abs(),
            pnl_unrealised: Decimal::ZERO,
            pnl_realised: -trade.fees.fees,
            fees_enter: trade.fees.clone(),
            fees_exit: AssetFees::default(),
            time_enter: trade.time_exchange,
            time_exchange_update: trade.time_exchange,
            trades,
        }
    }
}

/// Represents a fully closed trading [`Position`] for a specific instrument.
///
/// Contains the final state and history of a [`Position`] that has been completely closed.
///
/// # Type Parameters
/// - `AssetKey`: The type representing the asset used for fees (e.g. AssetIndex, QuoteAsset, etc.)
/// - `InstrumentKey`: The type identifying the traded instrument (e.g. InstrumentIndex, etc.)
#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct PositionExited<AssetKey, InstrumentKey = InstrumentIndex> {
    /// Closed [`Position`] Instrument identifier (eg/ InstrumentIndex, InstrumentNameInternal, etc.).
    pub instrument: InstrumentKey,

    /// Closed [`Position`] direction (Side::Buy => LONG, Side::Sell => SHORT).
    pub side: Side,

    /// Volume-weighted average entry price across all [`Position`] increasing [`Trade`]s.
    pub price_entry_average: Decimal,

    /// Maximum absolute [`Position`] quantity reached by all entry/increase [`Trade`]s.
    pub quantity_abs_max: Decimal,

    /// Cumulative realised PnL from closing the full [`Position`] `quantity_abs_max`.
    ///
    /// Note this includes fees.
    pub pnl_realised: Decimal,

    /// Cumulative fees paid when entering the [`Position`].
    pub fees_enter: AssetFees<AssetKey>,

    /// Cumulative fees paid when exiting the [`Position`].
    pub fees_exit: AssetFees<AssetKey>,

    /// Timestamp of [`Trade`] that triggered the initial [`Position`] entry.
    pub time_enter: DateTime<Utc>,

    /// Timestamp of [`Trade`] that triggered the closing of the [`Position`].
    pub time_exit: DateTime<Utc>,

    /// [`TradeId`]s of all the [`Trade`]s associated with the closed [`Position`].
    pub trades: Vec<TradeId>,
}

impl<AssetKey, InstrumentKey> From<Position<AssetKey, InstrumentKey>>
    for PositionExited<AssetKey, InstrumentKey>
{
    fn from(value: Position<AssetKey, InstrumentKey>) -> Self {
        Self {
            instrument: value.instrument,
            side: value.side,
            price_entry_average: value.price_entry_average,
            quantity_abs_max: value.quantity_abs_max,
            pnl_realised: value.pnl_realised,
            fees_enter: value.fees_enter,
            fees_exit: value.fees_exit,
            time_enter: value.time_enter,
            time_exit: value.time_exchange_update,
            trades: value.trades,
        }
    }
}

/// Calculates the volume-weighted average entry price when adding a [`Trade`] data to existing
/// [`Position`] data.
///
/// This function uses the formula: <br>
/// (current_value + trade_value) / (current_quantity + trade_quantity)
///
/// # Arguments
/// * `current_price_entry_average` - The current average entry price of the position
/// * `current_quantity_abs` - The current absolute quantity of the position
/// * `trade_price` - The price of the new trade
/// * `trade_quantity_abs` - The absolute quantity of the new trade
fn calculate_price_entry_average(
    current_price_entry_average: Decimal,
    current_quantity_abs: Decimal,
    trade_price: Decimal,
    trade_quantity_abs: Decimal,
) -> Decimal {
    if current_quantity_abs.is_zero() && trade_quantity_abs.is_zero() {
        return Decimal::ZERO;
    }

    let current_value = current_price_entry_average * current_quantity_abs;
    let trade_value = trade_price * trade_quantity_abs;

    (current_value + trade_value) / (current_quantity_abs + trade_quantity_abs)
}

/// Calculate the estimated unrealised PnL from closing a [`Position`] `quantity_abs` at the
/// provided price.
pub fn calculate_pnl_unrealised(
    position_side: Side,
    price_entry_average: Decimal,
    quantity_abs: Decimal,
    quantity_abs_max: Decimal,
    fees_enter: Decimal,
    price: Decimal,
) -> Decimal {
    let approx_exit_fees =
        approximate_remaining_exit_fees(quantity_abs, quantity_abs_max, fees_enter);

    let value_quote_current = quantity_abs * price;
    let value_quote_entry = quantity_abs * price_entry_average;

    match position_side {
        Side::Buy => value_quote_current - value_quote_entry - approx_exit_fees,
        Side::Sell => value_quote_entry - value_quote_current - approx_exit_fees,
    }
}

/// Approximate the exit fees from closing a [`Position`] with `quantity_abs`.
///
/// The `fees_enter` value was the fee cost to enter a [`Position`] of `quantity_abs_max`,
/// therefore this 'fee per quantity' ratio can be used to approximate the exit fees required to
/// close a `quantity_abs` [`Position`].
fn approximate_remaining_exit_fees(
    quantity_abs: Decimal,
    quantity_abs_max: Decimal,
    fees_enter: Decimal,
) -> Decimal {
    (quantity_abs / quantity_abs_max) * fees_enter
}

/// Calculate the realised PnL generated from closing the provided [`Position`] quantity, at the
/// specified price and closing fee.
pub fn calculate_pnl_realised(
    position_side: Side,
    price_entry_average: Decimal,
    closed_quantity: Decimal,
    closed_price: Decimal,
    closed_fee: Decimal,
) -> Decimal {
    let close_quantity = closed_quantity.abs();
    let value_quote_closed = close_quantity * closed_price;
    let value_quote_entry = close_quantity * price_entry_average;

    match position_side {
        Side::Buy => value_quote_closed - value_quote_entry - closed_fee,
        Side::Sell => value_quote_entry - value_quote_closed - closed_fee,
    }
}

/// Calculate the PnL returns.
///
/// Returns = pnl_realised / cost_of_investment
///
/// See docs: <https://www.investopedia.com/articles/basics/10/guide-to-calculating-roi.asp>
pub fn calculate_pnl_return(
    pnl_realised: Decimal,
    price_entry_average: Decimal,
    quantity_abs_max: Decimal,
) -> Decimal {
    pnl_realised / (price_entry_average * quantity_abs_max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{time_plus_days, trade};
    use barter_instrument::instrument::name::InstrumentNameInternal;
    use rust_decimal_macros::dec;

    #[test]
    fn test_position_update_from_trade() {
        struct TestCase {
            initial_trade: Trade<QuoteAsset, InstrumentNameInternal>,
            update_trade: Trade<QuoteAsset, InstrumentNameInternal>,
            expected_position: Option<Position<QuoteAsset, InstrumentNameInternal>>,
            expected_position_exited: Option<PositionExited<QuoteAsset, InstrumentNameInternal>>,
        }

        let base_time = DateTime::<Utc>::MIN_UTC;

        let cases = vec![
            // TC0: Increase long position
            TestCase {
                initial_trade: trade(base_time, Side::Buy, 100.0, 1.0, 10.0),
                update_trade: trade(time_plus_days(base_time, 1), Side::Buy, 120.0, 1.0, 10.0),
                expected_position: Some(Position {
                    instrument: InstrumentNameInternal::new("instrument"),
                    side: Side::Buy,
                    price_entry_average: dec!(110.0),
                    quantity_abs: dec!(2.0),
                    quantity_abs_max: dec!(2.0),
                    pnl_unrealised: dec!(0.0),
                    pnl_realised: dec!(-20.0), // Sum of fees
                    fees_enter: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(20.0),
                    },
                    fees_exit: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(0.0),
                    },
                    time_enter: base_time,
                    time_exchange_update: time_plus_days(base_time, 1),
                    trades: vec![TradeId::new("trade_id"), TradeId::new("trade_id")],
                }),
                expected_position_exited: None,
            },
            // TC1: Partial reduce long position
            TestCase {
                initial_trade: trade(base_time, Side::Buy, 100.0, 2.0, 10.0),
                update_trade: trade(time_plus_days(base_time, 1), Side::Sell, 150.0, 0.5, 5.0),
                expected_position: Some(Position {
                    instrument: InstrumentNameInternal::new("instrument"),
                    side: Side::Buy,
                    price_entry_average: dec!(100.0), // update_trade is Sell, so unchanged
                    quantity_abs: dec!(1.5),
                    quantity_abs_max: dec!(2.0),
                    pnl_unrealised: dec!(67.5), // (150-100)*(2.0-0.5) - approx_exit_fees (1.5/2 * 10)
                    pnl_realised: dec!(10.0),   // (150-100)*0.5 - 15_fees
                    fees_enter: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(10.0),
                    },
                    fees_exit: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(5.0),
                    },
                    time_enter: base_time,
                    time_exchange_update: time_plus_days(base_time, 1),
                    trades: vec![TradeId::new("trade_id"), TradeId::new("trade_id")],
                }),
                expected_position_exited: None,
            },
            // TC2: Exact position close, in profit
            TestCase {
                initial_trade: trade(base_time, Side::Buy, 100.0, 1.0, 10.0),
                update_trade: trade(time_plus_days(base_time, 1), Side::Sell, 150.0, 1.0, 10.0),
                expected_position: None,
                expected_position_exited: Some(PositionExited {
                    instrument: InstrumentNameInternal::new("instrument"),
                    side: Side::Buy,
                    price_entry_average: dec!(100.0),
                    quantity_abs_max: dec!(1.0),
                    pnl_realised: dec!(30.0), // (150-100)*1 - 20 (total fees)
                    fees_enter: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(10.0),
                    },
                    fees_exit: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(10.0),
                    },
                    time_enter: base_time,
                    time_exit: time_plus_days(base_time, 1),
                    trades: vec![TradeId::new("trade_id"), TradeId::new("trade_id")],
                }),
            },
            // TC3: Position flip (close and open new)
            TestCase {
                initial_trade: trade(base_time, Side::Buy, 100.0, 1.0, 10.0),
                update_trade: trade(time_plus_days(base_time, 1), Side::Sell, 150.0, 2.0, 20.0),
                expected_position: Some(Position {
                    instrument: InstrumentNameInternal::new("instrument"),
                    side: Side::Sell,
                    price_entry_average: dec!(150.0),
                    quantity_abs: dec!(1.0),
                    quantity_abs_max: dec!(1.0),
                    pnl_unrealised: dec!(0.0),
                    pnl_realised: dec!(-10.0), // Entry fees for new position (2-1)*(1/2)*20
                    fees_enter: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(10.0),
                    },
                    fees_exit: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(0.0),
                    },
                    time_enter: time_plus_days(base_time, 1),
                    time_exchange_update: time_plus_days(base_time, 1),
                    trades: vec![TradeId::new("trade_id")],
                }),
                expected_position_exited: Some(PositionExited {
                    instrument: InstrumentNameInternal::new("instrument"),
                    side: Side::Buy,
                    price_entry_average: dec!(100.0),
                    quantity_abs_max: dec!(1.0),
                    pnl_realised: dec!(30.0), // (150-100)*1 - 20 (total fees)
                    fees_enter: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(10.0),
                    },
                    fees_exit: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(10.0),
                    },
                    time_enter: base_time,
                    time_exit: time_plus_days(base_time, 1),
                    trades: vec![TradeId::new("trade_id"), TradeId::new("trade_id")],
                }),
            },
            // TC4: Increase short position
            TestCase {
                initial_trade: trade(base_time, Side::Sell, 100.0, 1.0, 10.0),
                update_trade: trade(base_time, Side::Sell, 80.0, 1.0, 10.0),
                expected_position: Some(Position {
                    instrument: InstrumentNameInternal::new("instrument"),
                    side: Side::Sell,
                    price_entry_average: dec!(90.0), // (100*1 + 80*1)/(1 + 1)
                    quantity_abs: dec!(2.0),
                    quantity_abs_max: dec!(2.0),
                    pnl_unrealised: dec!(0.0), // (90-80)*2 - approx_exit_fees(2/2 * 20)
                    pnl_realised: dec!(-20.0), // Sum of entry fees
                    fees_enter: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(20.0),
                    },
                    fees_exit: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(0.0),
                    },
                    time_enter: base_time,
                    time_exchange_update: base_time,
                    trades: vec![TradeId::new("trade_id"), TradeId::new("trade_id")],
                }),
                expected_position_exited: None,
            },
            // TC5: Partial reduce short position
            TestCase {
                initial_trade: trade(base_time, Side::Sell, 100.0, 2.0, 10.0),
                update_trade: trade(base_time, Side::Buy, 80.0, 0.5, 5.0),
                expected_position: Some(Position {
                    instrument: InstrumentNameInternal::new("instrument"),
                    side: Side::Sell,
                    price_entry_average: dec!(100.0), // update_trade is Buy, so unchanged
                    quantity_abs: dec!(1.5),
                    quantity_abs_max: dec!(2.0),
                    pnl_unrealised: dec!(22.5), // (100-80)*1.5 - approx_exit_fees(1.5/2 * 10)
                    pnl_realised: dec!(-5.0),   // 10_fee_entry - (100-80)*0.5 - 5_fee_exit
                    fees_enter: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(10.0),
                    },
                    fees_exit: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(5.0),
                    },
                    time_enter: base_time,
                    time_exchange_update: base_time,
                    trades: vec![TradeId::new("trade_id"), TradeId::new("trade_id")],
                }),
                expected_position_exited: None,
            },
            // TC6: Exact short position close
            TestCase {
                initial_trade: trade(base_time, Side::Sell, 100.0, 1.0, 10.0),
                update_trade: trade(base_time, Side::Buy, 80.0, 1.0, 10.0),
                expected_position: None,
                expected_position_exited: Some(PositionExited {
                    instrument: InstrumentNameInternal::new("instrument"),
                    side: Side::Sell,
                    price_entry_average: dec!(100.0),
                    quantity_abs_max: dec!(1.0),
                    pnl_realised: dec!(0.0), // (100-80)*1 - 20 (total fees)
                    fees_enter: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(10.0),
                    },
                    fees_exit: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(10.0),
                    },
                    time_enter: base_time,
                    time_exit: base_time,
                    trades: vec![TradeId::new("trade_id"), TradeId::new("trade_id")],
                }),
            },
            // TC7: Short position flip (close and open long)
            TestCase {
                initial_trade: trade(base_time, Side::Sell, 100.0, 1.0, 10.0),
                update_trade: trade(base_time, Side::Buy, 80.0, 2.0, 20.0),
                expected_position: Some(Position {
                    instrument: InstrumentNameInternal::new("instrument"),
                    side: Side::Buy,
                    price_entry_average: dec!(80.0),
                    quantity_abs: dec!(1.0),
                    quantity_abs_max: dec!(1.0),
                    pnl_unrealised: dec!(0.0),
                    pnl_realised: dec!(-10.0), // Entry fees for new position
                    fees_enter: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(10.0),
                    },
                    fees_exit: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(0.0),
                    },
                    time_enter: base_time,
                    time_exchange_update: base_time,
                    trades: vec![TradeId::new("trade_id")],
                }),
                expected_position_exited: Some(PositionExited {
                    instrument: InstrumentNameInternal::new("instrument"),
                    side: Side::Sell,
                    price_entry_average: dec!(100.0),
                    quantity_abs_max: dec!(1.0),
                    pnl_realised: dec!(0.0), // (100-80)*1 - 20 (total fees)
                    fees_enter: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(10.0),
                    },
                    fees_exit: AssetFees {
                        asset: QuoteAsset,
                        fees: dec!(10.0),
                    },
                    time_enter: base_time,
                    time_exit: base_time,
                    trades: vec![TradeId::new("trade_id"), TradeId::new("trade_id")],
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let position = Position::from(&test.initial_trade);
            let (updated_position, exited_position) =
                position.update_from_trade(&test.update_trade);

            assert_eq!(updated_position, test.expected_position, "TC{index} failed");
            assert_eq!(
                exited_position, test.expected_position_exited,
                "TC{index} failed"
            );
        }
    }

    #[test]
    fn test_calculate_price_entry_average() {
        struct TestCase {
            current_price_entry_average: Decimal,
            current_quantity_abs: Decimal,
            trade_price: Decimal,
            trade_quantity_abs: Decimal,
            expected: Decimal,
        }

        let cases = vec![
            // TC0: equal contribution
            TestCase {
                current_price_entry_average: dec!(100.0),
                current_quantity_abs: dec!(2.0),
                trade_price: dec!(200.0),
                trade_quantity_abs: dec!(2.0),
                expected: dec!(150.0),
            },
            // TC1: trade larger contribution
            TestCase {
                current_price_entry_average: dec!(100.0),
                current_quantity_abs: dec!(2.0),
                trade_price: dec!(200.0),
                trade_quantity_abs: dec!(4.0),
                expected: dec!(166.66666666666666666666666667),
            },
            // TC2: current larger contribution
            TestCase {
                current_price_entry_average: dec!(100.0),
                current_quantity_abs: dec!(20.0),
                trade_price: dec!(200.0),
                trade_quantity_abs: dec!(1.0),
                expected: dec!(104.76190476190476190476190476),
            },
            // TC3: zero current quantity, so expect trade price
            TestCase {
                current_price_entry_average: dec!(100.0),
                current_quantity_abs: dec!(0.0),
                trade_price: dec!(200.0),
                trade_quantity_abs: dec!(4.0),
                expected: dec!(200.0),
            },
            // TC4: zero trade quantity, so expect current price
            TestCase {
                current_price_entry_average: dec!(100.0),
                current_quantity_abs: dec!(10.0),
                trade_price: dec!(0.0),
                trade_quantity_abs: dec!(0.0),
                expected: dec!(100.0),
            },
            // TC5: both zero quantities
            TestCase {
                current_price_entry_average: dec!(100.0),
                current_quantity_abs: dec!(0.0),
                trade_price: dec!(200.0),
                trade_quantity_abs: dec!(0.0),
                expected: dec!(0.0),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = calculate_price_entry_average(
                test.current_price_entry_average,
                test.current_quantity_abs,
                test.trade_price,
                test.trade_quantity_abs,
            );

            assert_eq!(actual, test.expected, "TC{} failed", index)
        }
    }

    #[test]
    fn test_calculate_pnl_unrealised() {
        struct TestCase {
            position_side: Side,
            price_entry_average: Decimal,
            quantity_abs: Decimal,
            quantity_abs_max: Decimal,
            fees_enter: Decimal,
            price: Decimal,
            expected: Decimal,
        }

        let cases = vec![
            // TC0: LONG position in profit
            TestCase {
                position_side: Side::Buy,
                price_entry_average: dec!(100.0),
                quantity_abs: dec!(1.0),
                quantity_abs_max: dec!(1.0),
                fees_enter: dec!(10.0),
                price: dec!(150.0),
                expected: dec!(40.0), // (150-100)*1 - 10
            },
            // TC1: LONG position at loss
            TestCase {
                position_side: Side::Buy,
                price_entry_average: dec!(100.0),
                quantity_abs: dec!(1.0),
                quantity_abs_max: dec!(1.0),
                fees_enter: dec!(10.0),
                price: dec!(80.0),
                expected: dec!(-30.0), // (80-100)*1 - 10
            },
            // TC2: SHORT position in profit
            TestCase {
                position_side: Side::Sell,
                price_entry_average: dec!(100.0),
                quantity_abs: dec!(1.0),
                quantity_abs_max: dec!(1.0),
                fees_enter: dec!(10.0),
                price: dec!(80.0),
                expected: dec!(10.0), // (100-80)*1 - 10
            },
            // TC3: SHORT position at loss
            TestCase {
                position_side: Side::Sell,
                price_entry_average: dec!(100.0),
                quantity_abs: dec!(1.0),
                quantity_abs_max: dec!(1.0),
                fees_enter: dec!(10.0),
                price: dec!(150.0),
                expected: dec!(-60.0), // (100-150)*1 - 10
            },
            // TC4: Partial position remaining (half closed)
            TestCase {
                position_side: Side::Buy,
                price_entry_average: dec!(100.0),
                quantity_abs: dec!(0.5),
                quantity_abs_max: dec!(1.0),
                fees_enter: dec!(10.0),
                price: dec!(150.0),
                expected: dec!(20.0), // (150-100)*0.5 - (0.5/1.0)*10
            },
            // TC5: Zero quantity position
            TestCase {
                position_side: Side::Buy,
                price_entry_average: dec!(100.0),
                quantity_abs: dec!(0.0),
                quantity_abs_max: dec!(1.0),
                fees_enter: dec!(10.0),
                price: dec!(150.0),
                expected: dec!(0.0),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = calculate_pnl_unrealised(
                test.position_side,
                test.price_entry_average,
                test.quantity_abs,
                test.quantity_abs_max,
                test.fees_enter,
                test.price,
            );

            assert_eq!(actual, test.expected, "TC{} failed", index);
        }
    }

    #[test]
    fn test_approximate_remaining_exit_fees() {
        struct TestCase {
            quantity_abs: Decimal,
            quantity_abs_max: Decimal,
            fees_enter: Decimal,
            expected: Decimal,
        }

        let cases = vec![
            // TC0: Full position - expect full fees
            TestCase {
                quantity_abs: dec!(1.0),
                quantity_abs_max: dec!(1.0),
                fees_enter: dec!(10.0),
                expected: dec!(10.0),
            },
            // TC1: Half position - expect half fees
            TestCase {
                quantity_abs: dec!(0.5),
                quantity_abs_max: dec!(1.0),
                fees_enter: dec!(10.0),
                expected: dec!(5.0),
            },
            // TC2: Zero position - expect zero fees
            TestCase {
                quantity_abs: dec!(0.0),
                quantity_abs_max: dec!(1.0),
                fees_enter: dec!(10.0),
                expected: dec!(0.0),
            },
            // TC3: Larger current quantity than max (edge case)
            TestCase {
                quantity_abs: dec!(2.0),
                quantity_abs_max: dec!(1.0),
                fees_enter: dec!(10.0),
                expected: dec!(20.0),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = approximate_remaining_exit_fees(
                test.quantity_abs,
                test.quantity_abs_max,
                test.fees_enter,
            );

            assert_eq!(actual, test.expected, "TC{} failed", index);
        }
    }

    #[test]
    fn test_calculate_pnl_realised() {
        struct TestCase {
            side: Side,
            price_entry_average: Decimal,
            closed_quantity: Decimal,
            closed_price: Decimal,
            closed_fee: Decimal,
            expected: Decimal,
        }

        let cases = vec![
            // TC0: LONG in profit w/ fee deduction
            TestCase {
                side: Side::Buy,
                price_entry_average: dec!(100.0),
                closed_quantity: dec!(10.0),
                closed_price: dec!(150.0),
                closed_fee: dec!(5.0),
                expected: dec!(495.0),
            },
            // TC1: LONG in profit w/o fee deduction
            TestCase {
                side: Side::Buy,
                price_entry_average: dec!(100.0),
                closed_quantity: dec!(10.0),
                closed_price: dec!(150.0),
                closed_fee: dec!(0.0),
                expected: dec!(500.0),
            },
            // TC2: LONG in profit w/ fee rebate
            TestCase {
                side: Side::Buy,
                price_entry_average: dec!(100.0),
                closed_quantity: dec!(10.0),
                closed_price: dec!(150.0),
                closed_fee: dec!(-5.0),
                expected: dec!(505.0),
            },
            // TC3: LONG in loss w/ fee deduction
            TestCase {
                side: Side::Buy,
                price_entry_average: dec!(100.0),
                closed_quantity: dec!(10.0),
                closed_price: dec!(50.0),
                closed_fee: dec!(5.0),
                expected: dec!(-505.0),
            },
            // TC4: LONG in loss w/o fee deduction
            TestCase {
                side: Side::Buy,
                price_entry_average: dec!(100.0),
                closed_quantity: dec!(10.0),
                closed_price: dec!(50.0),
                closed_fee: dec!(0.0),
                expected: dec!(-500.0),
            },
            // TC5: LONG in loss w/ fee rebate
            TestCase {
                side: Side::Buy,
                price_entry_average: dec!(100.0),
                closed_quantity: dec!(10.0),
                closed_price: dec!(50.0),
                closed_fee: dec!(-5.0),
                expected: dec!(-495.0),
            },
            // TC6: SHORT in profit w/ fee deduction
            TestCase {
                side: Side::Sell,
                price_entry_average: dec!(100.0),
                closed_quantity: dec!(10.0),
                closed_price: dec!(50.0),
                closed_fee: dec!(5.0),
                expected: dec!(495.0),
            },
            // TC7: SHORT in profit w/o fee deduction
            TestCase {
                side: Side::Sell,
                price_entry_average: dec!(100.0),
                closed_quantity: dec!(10.0),
                closed_price: dec!(50.0),
                closed_fee: dec!(0.0),
                expected: dec!(500.0),
            },
            // TC8: SHORT in profit w/ fee rebate
            TestCase {
                side: Side::Sell,
                price_entry_average: dec!(100.0),
                closed_quantity: dec!(10.0),
                closed_price: dec!(50.0),
                closed_fee: dec!(-5.0),
                expected: dec!(505.0),
            },
            // TC9: SHORT in loss w/ fee deduction
            TestCase {
                side: Side::Sell,
                price_entry_average: dec!(100.0),
                closed_quantity: dec!(10.0),
                closed_price: dec!(150.0),
                closed_fee: dec!(5.0),
                expected: dec!(-505.0),
            },
            // TC10: SHORT in loss w/o fee deduction
            TestCase {
                side: Side::Sell,
                price_entry_average: dec!(100.0),
                closed_quantity: dec!(10.0),
                closed_price: dec!(150.0),
                closed_fee: dec!(0.0),
                expected: dec!(-500.0),
            },
            // TC10: SHORT in loss w/ fee rebate
            TestCase {
                side: Side::Sell,
                price_entry_average: dec!(100.0),
                closed_quantity: dec!(10.0),
                closed_price: dec!(150.0),
                closed_fee: dec!(-5.0),
                expected: dec!(-495.0),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = calculate_pnl_realised(
                test.side,
                test.price_entry_average.into(),
                test.closed_quantity.into(),
                test.closed_price.into(),
                test.closed_fee.into(),
            );

            assert_eq!(actual, test.expected, "TC{} failed", index);
        }
    }

    #[test]
    fn test_calculate_pnl_return() {
        struct TestCase {
            pnl_realised: Decimal,
            price_entry_average: Decimal,
            quantity_abs_max: Decimal,
            expected: Decimal,
        }

        let cases = vec![
            // TC0: Break even (0% return)
            TestCase {
                pnl_realised: dec!(0.0),
                price_entry_average: dec!(100.0),
                quantity_abs_max: dec!(1.0),
                expected: dec!(0.0),
            },
            // TC1: 100% return
            TestCase {
                pnl_realised: dec!(100.0),
                price_entry_average: dec!(100.0),
                quantity_abs_max: dec!(1.0),
                expected: dec!(1.0),
            },
            // TC2: -50% return
            TestCase {
                pnl_realised: dec!(-50.0),
                price_entry_average: dec!(100.0),
                quantity_abs_max: dec!(1.0),
                expected: dec!(-0.5),
            },
            // TC3: Complex case with larger position
            TestCase {
                pnl_realised: dec!(500.0),
                price_entry_average: dec!(100.0),
                quantity_abs_max: dec!(10.0),
                expected: dec!(0.5), // 500/(100*10)
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = calculate_pnl_return(
                test.pnl_realised.into(),
                test.price_entry_average.into(),
                test.quantity_abs_max.into(),
            );

            assert_eq!(actual, test.expected, "TC{} failed", index);
        }
    }
}
