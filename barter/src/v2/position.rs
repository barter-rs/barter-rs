use crate::v2::trade::{AssetFees, Trade, TradeId};
use barter_instrument::Side;
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use rust_decimal::prelude::Zero;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct Position<AssetKey, InstrumentKey> {
    pub instrument: InstrumentKey,
    pub side: Side,
    pub price_entry_average: f64,
    pub quantity_abs: f64,
    pub quantity_abs_max: f64,
    pub pnl_unrealised: f64,
    pub pnl_realised: f64,
    pub fees_enter: AssetFees<AssetKey>,
    pub fees_exit: AssetFees<AssetKey>,
    pub time_enter: DateTime<Utc>,
    pub time_exchange_update: DateTime<Utc>,
    pub trades: Vec<TradeId>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct PositionExchange<InstrumentKey> {
    pub instrument: InstrumentKey,
    pub side: Side,
    pub price_entry_average: f64,
    pub quantity_abs: f64,
    pub time_exchange_update: DateTime<Utc>,
}

impl<InstrumentKey> PositionExchange<InstrumentKey> {
    pub fn new_flat(instrument: InstrumentKey) -> Self {
        Self {
            instrument,
            side: Side::Buy,
            price_entry_average: 0.0,
            quantity_abs: 0.0,
            time_exchange_update: DateTime::<Utc>::MIN_UTC,
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct PositionExited<AssetKey, InstrumentKey> {
    pub instrument: InstrumentKey,
    pub side: Side,
    pub price_entry_average: f64,
    pub quantity_abs_max: f64,
    pub pnl_realised: f64,
    pub fees_enter: AssetFees<AssetKey>,
    pub fees_exit: AssetFees<AssetKey>,
    pub time_enter: DateTime<Utc>,
    pub time_exit: DateTime<Utc>,
    pub trades: Vec<TradeId>,
}

impl<AssetKey, InstrumentKey> Position<AssetKey, InstrumentKey> {
    pub fn update_from_trade(
        mut self,
        trade: &Trade<AssetKey, InstrumentKey>,
    ) -> (
        Option<Self>,
        Option<PositionExited<AssetKey, InstrumentKey>>,
    )
    where
        AssetKey: Debug + Clone + PartialEq,
        InstrumentKey: Debug + Clone + PartialEq,
    {
        assert_eq!(
            self.instrument, trade.instrument,
            "Position should never be updated from a trade for a different Instrument"
        );
        assert_eq!(
            self.fees_enter.asset, trade.fees.asset,
            "Position fees Asset should never be different from trade fees Asset"
        );

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
                self.quantity_abs -= trade.quantity.abs();
                self.fees_exit.fees += trade.fees.fees;
                self.time_exchange_update = trade.time_exchange;
                self.update_pnl_realised(trade.quantity, trade.price, trade.fees.fees);
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
                    instrument: trade.instrument.clone(),
                    order_id: trade.order_id.clone(),
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
                self.quantity_abs = 0.0;
                self.update_pnl_unrealised(trade.price);

                (
                    Some(Self::from(&next_position_trade)),
                    Some(PositionExited::from(self)),
                )
            }
            _ => unreachable!("match expression guard statements cover all cases"),
        }
    }

    pub fn update_price_entry_average(&mut self, trade: &Trade<AssetKey, InstrumentKey>) {
        self.price_entry_average = calculate_price_entry_average(
            self.price_entry_average,
            self.quantity_abs,
            trade.price,
            trade.quantity.abs(),
        );
    }

    /// Update [`Position::pnl_unrealised`](Position) with the estimated PnL from closing
    /// the [`Position`] at the provided price.
    pub fn update_pnl_unrealised(&mut self, price: f64) {
        self.pnl_unrealised = calculate_pnl_unrealised(
            self.side,
            self.price_entry_average,
            self.quantity_abs,
            self.quantity_abs_max,
            self.fees_enter.fees,
            price,
        );
    }

    /// Update [`Position::pnl_realised`](Position) with the PnL generated from closing the
    /// provided quantity, at the provided price and closing fee.
    pub fn update_pnl_realised(
        &mut self,
        closed_quantity: f64,
        closed_price: f64,
        closed_fee: f64,
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

impl<AssetKey, InstrumentKey> From<&Trade<AssetKey, InstrumentKey>>
    for Position<AssetKey, InstrumentKey>
where
    AssetKey: Clone,
    InstrumentKey: Clone,
{
    fn from(trade: &Trade<AssetKey, InstrumentKey>) -> Self {
        let mut trades = Vec::with_capacity(2);
        trades.push(trade.id.clone());
        Self {
            instrument: trade.instrument.clone(),
            side: trade.side,
            price_entry_average: trade.price,
            quantity_abs: trade.quantity.abs(),
            quantity_abs_max: trade.quantity.abs(),
            pnl_unrealised: 0.0,
            pnl_realised: -trade.fees.fees,
            fees_enter: trade.fees.clone(),
            fees_exit: AssetFees::default(),
            time_enter: trade.time_exchange,
            time_exchange_update: trade.time_exchange,
            trades,
        }
    }
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

fn calculate_price_entry_average(
    current_price_entry_average: f64,
    current_quantity_abs: f64,
    trade_price: f64,
    trade_quantity_abs: f64,
) -> f64 {
    // Todo: perhaps make this more robust, eg/ with typed_floats
    if current_quantity_abs.is_zero() && trade_quantity_abs.is_zero() {
        return 0.0;
    }

    let current_value = current_price_entry_average * current_quantity_abs;
    let trade_value = trade_price * trade_quantity_abs;

    (current_value + trade_value) / (current_quantity_abs + trade_quantity_abs)
}

/// Calculate the estimated unrealised PnL from closing a [`Position`] `quantity_abs` at the
/// provided price.
pub fn calculate_pnl_unrealised(
    position_side: Side,
    price_entry_average: f64,
    quantity_abs: f64,
    quantity_abs_max: f64,
    fees_enter: f64,
    price: f64,
) -> f64 {
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
    quantity_abs: f64,
    quantity_abs_max: f64,
    fees_enter: f64,
) -> f64 {
    (quantity_abs / quantity_abs_max) * fees_enter
}

/// Calculate the realised PnL generated from closing the provided [`Position`] quantity, at the
/// specified price and closing fee.
pub fn calculate_pnl_realised(
    position_side: Side,
    price_entry_average: f64,
    closed_quantity: f64,
    closed_price: f64,
    closed_fee: f64,
) -> f64 {
    let close_quantity = closed_quantity.abs();
    let value_quote_closed = close_quantity * closed_price;
    let value_quote_entry = close_quantity * price_entry_average;

    match position_side {
        Side::Buy => value_quote_closed - value_quote_entry - closed_fee,
        Side::Sell => value_quote_entry - value_quote_closed - closed_fee,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_price_entry_average() {
        struct TestCase {
            current_price_entry_average: f64,
            current_quantity_abs: f64,
            trade_price: f64,
            trade_quantity_abs: f64,
            expected: f64,
        }

        let cases = vec![
            // TC0: equal contribution
            TestCase {
                current_price_entry_average: 100.0,
                current_quantity_abs: 2.0,
                trade_price: 200.0,
                trade_quantity_abs: 2.0,
                expected: 150.0,
            },
            // TC1: trade larger contribution
            TestCase {
                current_price_entry_average: 100.0,
                current_quantity_abs: 2.0,
                trade_price: 200.0,
                trade_quantity_abs: 4.0,
                expected: 166.666666666666666666,
            },
            // TC2: current larger contribution
            TestCase {
                current_price_entry_average: 100.0,
                current_quantity_abs: 20.0,
                trade_price: 200.0,
                trade_quantity_abs: 1.0,
                expected: 104.762,
            },
            // TC3: zero current quantity, so expect trade price
            TestCase {
                current_price_entry_average: 100.0,
                current_quantity_abs: 0.0,
                trade_price: 200.0,
                trade_quantity_abs: 4.0,
                expected: 200.0,
            },
            // TC4: zero trade quantity, so expect current price
            TestCase {
                current_price_entry_average: 100.0,
                current_quantity_abs: 10.0,
                trade_price: 0.0,
                trade_quantity_abs: 4.0,
                expected: 100.0,
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = calculate_price_entry_average(
                test.current_price_entry_average,
                test.current_quantity_abs,
                test.trade_price,
                test.trade_quantity_abs,
            );

            assert!((actual - test.expected).abs() < 0.001, "TC{} failed", index)
        }
    }

    #[test]
    fn test_calculate_pnl_unrealised() {
        todo!()
    }

    #[test]
    fn test_approximate_remaining_exit_fees() {
        todo!()
    }

    #[test]
    fn test_calculate_pnl_realised() {
        struct TestCase {
            side: Side,
            price_entry_average: f64,
            closed_quantity: f64,
            closed_price: f64,
            closed_fee: f64,
            expected: f64,
        }

        let cases = vec![
            // TC0: LONG in profit w/ fee deduction
            TestCase {
                side: Side::Buy,
                price_entry_average: 100.0,
                closed_quantity: 10.0,
                closed_price: 150.0,
                closed_fee: 5.0,
                expected: 495.0,
            },
            // TC1: LONG in profit w/o fee deduction
            TestCase {
                side: Side::Buy,
                price_entry_average: 100.0,
                closed_quantity: 10.0,
                closed_price: 150.0,
                closed_fee: 0.0,
                expected: 500.0,
            },
            // TC2: LONG in profit w/ fee rebate
            TestCase {
                side: Side::Buy,
                price_entry_average: 100.0,
                closed_quantity: 10.0,
                closed_price: 150.0,
                closed_fee: -5.0,
                expected: 505.0,
            },
            // TC3: LONG in loss w/ fee deduction
            TestCase {
                side: Side::Buy,
                price_entry_average: 100.0,
                closed_quantity: 10.0,
                closed_price: 50.0,
                closed_fee: 5.0,
                expected: -505.0,
            },
            // TC4: LONG in loss w/o fee deduction
            TestCase {
                side: Side::Buy,
                price_entry_average: 100.0,
                closed_quantity: 10.0,
                closed_price: 50.0,
                closed_fee: 0.0,
                expected: -500.0,
            },
            // TC5: LONG in loss w/ fee rebate
            TestCase {
                side: Side::Buy,
                price_entry_average: 100.0,
                closed_quantity: 10.0,
                closed_price: 50.0,
                closed_fee: -5.0,
                expected: -495.0,
            },
            // TC6: SHORT in profit w/ fee deduction
            TestCase {
                side: Side::Sell,
                price_entry_average: 100.0,
                closed_quantity: 10.0,
                closed_price: 50.0,
                closed_fee: 5.0,
                expected: 495.0,
            },
            // TC7: SHORT in profit w/o fee deduction
            TestCase {
                side: Side::Sell,
                price_entry_average: 100.0,
                closed_quantity: 10.0,
                closed_price: 50.0,
                closed_fee: 0.0,
                expected: 500.0,
            },
            // TC8: SHORT in profit w/ fee rebate
            TestCase {
                side: Side::Sell,
                price_entry_average: 100.0,
                closed_quantity: 10.0,
                closed_price: 50.0,
                closed_fee: -5.0,
                expected: 505.0,
            },
            // TC9: SHORT in loss w/ fee deduction
            TestCase {
                side: Side::Sell,
                price_entry_average: 100.0,
                closed_quantity: 10.0,
                closed_price: 150.0,
                closed_fee: 5.0,
                expected: -505.0,
            },
            // TC10: SHORT in loss w/o fee deduction
            TestCase {
                side: Side::Sell,
                price_entry_average: 100.0,
                closed_quantity: 10.0,
                closed_price: 150.0,
                closed_fee: 0.0,
                expected: -500.0,
            },
            // TC10: SHORT in loss w/ fee rebate
            TestCase {
                side: Side::Sell,
                price_entry_average: 100.0,
                closed_quantity: 10.0,
                closed_price: 150.0,
                closed_fee: -5.0,
                expected: -495.0,
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = calculate_pnl_realised(
                test.side,
                test.price_entry_average,
                test.closed_quantity,
                test.closed_price,
                test.closed_fee,
            );

            assert_eq!(actual, test.expected, "TC{} failed", index);
        }
    }
}
