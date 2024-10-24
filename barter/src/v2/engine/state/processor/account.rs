use crate::v2::{
    engine::state::{order_manager::OrderManager, EngineState, Updater},
    execution::{AccountEvent, AccountEventKind},
    Snapshot,
};
use barter_instrument::{asset::AssetIndex, instrument::InstrumentIndex};
use tracing::{info, warn};

impl<Market, Strategy, Risk> Updater<AccountEvent<AccountEventKind<AssetIndex, InstrumentIndex>>>
    for EngineState<AssetIndex, InstrumentIndex, Market, Strategy, Risk>
where
    Strategy: Updater<AccountEvent<AccountEventKind<AssetIndex, InstrumentIndex>>>,
    Risk: Updater<AccountEvent<AccountEventKind<AssetIndex, InstrumentIndex>>>,
{
    type Output = ();

    fn update(
        &mut self,
        event: &AccountEvent<AccountEventKind<AssetIndex, InstrumentIndex>>,
    ) -> Self::Output {
        info!(account = ?event, "EngineState updating from AccountEvent");
        let AccountEvent { exchange, kind } = event;
        match kind {
            AccountEventKind::Snapshot(snapshot) => {
                for balance in snapshot.balances {
                    self.assets
                        .state_by_index_mut(balance.asset)
                        .update_from_balance(Snapshot(&balance))
                }
                for instrument in snapshot.instruments {
                    self.instruments
                        .state_by_index_mut(instrument.position.instrument)
                        .update_from_account_snapshot(&instrument)
                }
            }
            AccountEventKind::BalanceSnapshot(balance) => {
                self.assets
                    .state_by_index_mut(balance.0.asset)
                    .update_from_balance(balance.as_ref());
            }
            AccountEventKind::PositionSnapshot(position) => {
                self.instruments
                    .state_by_index_mut(position.0.instrument)
                    .update_from_position_snapshot(position.as_ref());
            }
            AccountEventKind::OrderSnapshot(order) => {
                self.instruments.update_from_order_snapshot(order.as_ref())
            }
            AccountEventKind::OrderOpened(response) => {
                self.instruments.update_from_open(response)
            },
            AccountEventKind::OrderCancelled(response) => {
                self.instruments.update_from_cancel(response)
            }
            AccountEventKind::Trade(trade) => {
                self.instruments
                    .state_by_index_mut(trade.instrument)
                    .update_from_trade(trade);
            }
            AccountEventKind::ConnectivityError(error) => {
                warn!(%error, %exchange, "EngineState aware of Account ConnectivityError");
            }
        }

        // Update any user provided Strategy & Risk State
        self.strategy.update(event);
        self.risk.update(event);
    }
}
