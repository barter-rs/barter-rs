use crate::{
    model::{
        balance::{Balance, BalanceDelta, SymbolBalance},
        trade::Trade,
        AccountEvent, AccountEventKind,
    },
    ExecutionError, ExecutionId, Open, Order,
};
use barter_integration::model::{
    instrument::{symbol::Symbol, Instrument},
    Exchange, Side,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// [`ClientAccount`](super::ClientAccount) [`Balance`] for each [`Symbol`].
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct ClientBalances(pub HashMap<Symbol, Balance>);

impl ClientBalances {
    /// Return a reference to the [`Balance`] of the specified [`Symbol`].
    pub fn balance(&self, symbol: &Symbol) -> Result<&Balance, ExecutionError> {
        self.get(symbol).ok_or_else(|| {
            ExecutionError::Simulated(format!(
                "SimulatedExchange is not configured for Symbol: {symbol}"
            ))
        })
    }

    /// Return a mutable reference to the [`Balance`] of the specified [`Symbol`].
    pub fn balance_mut(&mut self, symbol: &Symbol) -> Result<&mut Balance, ExecutionError> {
        self.get_mut(symbol).ok_or_else(|| {
            ExecutionError::Simulated(format!(
                "SimulatedExchange is not configured for Symbol: {symbol}"
            ))
        })
    }

    /// Fetch the client [`Balance`] for every [`Symbol``].
    pub fn fetch_all(&self) -> Vec<SymbolBalance> {
        self.0
            .clone()
            .into_iter()
            .map(|(symbol, balance)| SymbolBalance::new(symbol, balance))
            .collect()
    }

    /// Determine if the client has sufficient available [`Balance`] to execute an
    /// [`Order<RequestOpen>`].
    pub fn has_sufficient_available_balance(
        &self,
        symbol: &Symbol,
        required_balance: f64,
    ) -> Result<(), ExecutionError> {
        let available = self.balance(symbol)?.available;
        match available >= required_balance {
            true => Ok(()),
            false => Err(ExecutionError::InsufficientBalance(symbol.clone())),
        }
    }

    /// Updates the associated [`Symbol`] [`Balance`] when a client creates an [`Order<Open>`]. The
    /// nature of the [`Balance`] change will depend on if the [`Order<Open>`] is a
    /// [`Side::Buy`] or [`Side::Sell`].
    pub fn update_from_open(&mut self, open: &Order<Open>, required_balance: f64) -> AccountEvent {
        let updated_balance = match open.side {
            Side::Buy => {
                let balance = self
                    .balance_mut(&open.instrument.quote)
                    .expect("Balance existence checked in has_sufficient_available_balance");

                balance.available -= required_balance;
                SymbolBalance::new(open.instrument.quote.clone(), *balance)
            }
            Side::Sell => {
                let balance = self
                    .balance_mut(&open.instrument.base)
                    .expect("Balance existence checked in has_sufficient_available_balance");

                balance.available -= required_balance;
                SymbolBalance::new(open.instrument.base.clone(), *balance)
            }
        };

        AccountEvent {
            received_time: Utc::now(),
            exchange: Exchange::from(ExecutionId::Simulated),
            kind: AccountEventKind::Balance(updated_balance),
        }
    }

    /// Updates the associated [`Symbol`] [`Balance`] when a client cancels an [`Order<Open>`]. The
    /// nature of the [`Balance`] change will depend on if the [`Order<Open>`] was a
    /// [`Side::Buy`] or [`Side::Sell`].
    pub fn update_from_cancel(&mut self, cancelled: &Order<Open>) -> SymbolBalance {
        match cancelled.side {
            Side::Buy => {
                let balance = self
                    .balance_mut(&cancelled.instrument.quote)
                    .expect("Balance existence checked when opening Order");

                balance.available += cancelled.state.price * cancelled.state.remaining_quantity();
                SymbolBalance::new(cancelled.instrument.quote.clone(), *balance)
            }
            Side::Sell => {
                let balance = self
                    .balance_mut(&cancelled.instrument.base)
                    .expect("Balance existence checked when opening Order");

                balance.available += cancelled.state.remaining_quantity();
                SymbolBalance::new(cancelled.instrument.base.clone(), *balance)
            }
        }
    }

    /// When a client [`Trade`] occurs, it causes a change in the [`Balance`] of the base & quote
    /// [`Symbol`]. The nature of each [`Balance`] change will depend on if the matched
    /// [`Order<Open>`] was a [`Side::Buy`] or [`Side::Sell`].
    ///
    /// A [`Side::Buy`] match causes the [`Symbol`] [`Balance`] of the base to increase by the
    /// `trade_quantity`, and the quote to decrease by the `trade_quantity * price`.
    ///
    /// A [`Side::Sell`] match causes the [`Symbol`] [`Balance`] of the base to decrease by the
    /// `trade_quantity`, and the quote to increase by the `trade_quantity * price`.
    pub fn update_from_trade(&mut self, trade: &Trade) -> AccountEvent {
        let Instrument { base, quote, .. } = &trade.instrument;

        // Calculate the base & quote Balance deltas
        let (base_delta, quote_delta) = match trade.side {
            Side::Buy => {
                // Base total & available increase by trade.quantity minus base trade.fees
                let base_increase = trade.quantity - trade.fees.fees;
                let base_delta = BalanceDelta {
                    total: base_increase,
                    available: base_increase,
                };

                // Quote total decreases by (trade.quantity * price)
                // Note: available was already decreased by the opening of the Side::Buy order
                let quote_delta = BalanceDelta {
                    total: -trade.quantity * trade.price,
                    available: 0.0,
                };

                (base_delta, quote_delta)
            }
            Side::Sell => {
                // Base total decreases by trade.quantity
                // Note: available was already decreased by the opening of the Side::Sell order
                let base_delta = BalanceDelta {
                    total: -trade.quantity,
                    available: 0.0,
                };

                // Quote total & available increase by (trade.quantity * price) minus quote fees
                let quote_increase = (trade.quantity * trade.price) - trade.fees.fees;
                let quote_delta = BalanceDelta {
                    total: quote_increase,
                    available: quote_increase,
                };

                (base_delta, quote_delta)
            }
        };

        // Apply BalanceDelta & return updated Balance
        let base_balance = self.update(base, base_delta);
        let quote_balance = self.update(quote, quote_delta);

        AccountEvent {
            received_time: Utc::now(),
            exchange: Exchange::from(ExecutionId::Simulated),
            kind: AccountEventKind::Balances(vec![
                SymbolBalance::new(base.clone(), base_balance),
                SymbolBalance::new(quote.clone(), quote_balance),
            ]),
        }
    }

    /// Apply the [`BalanceDelta`] to the [`Balance`] of the specified [`Symbol`], returning a
    /// `Copy` of the updated [`Balance`].
    pub fn update(&mut self, symbol: &Symbol, delta: BalanceDelta) -> Balance {
        let base_balance = self.balance_mut(symbol).unwrap();

        base_balance.apply(delta);

        *base_balance
    }
}

impl std::ops::Deref for ClientBalances {
    type Target = HashMap<Symbol, Balance>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ClientBalances {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
