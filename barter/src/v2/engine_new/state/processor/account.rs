use crate::v2::{
    engine_new::state::{order_manager::OrderManager, EngineState, Updater},
    execution::{AccountEvent, AccountEventKind},
    Snapshot,
};
use barter_instrument::{asset::AssetIndex, instrument::InstrumentIndex};
use tracing::{info, warn};

impl<Market, Strategy, Risk> Updater<AccountEvent<AccountEventKind<AssetIndex, InstrumentIndex>>>
    for EngineState<Market, Strategy, Risk>
where
    Strategy: Updater<AccountEvent<AccountEventKind<AssetIndex, InstrumentIndex>>>,
    Risk: Updater<AccountEvent<AccountEventKind<AssetIndex, InstrumentIndex>>>,
{
    type Output = ();

    fn update(
        &mut self,
        event: &AccountEvent<AccountEventKind<AssetIndex, InstrumentIndex>>,
    ) -> Self::Output {
        info!(account = ?event, "updating State from AccountEvent");

        // Update InstrumentState & BalanceState
        let AccountEvent { exchange, kind } = event;
        match kind {
            AccountEventKind::Snapshot(snapshot) => {
                self.assets
                    .update_from_balances(Snapshot(&snapshot.balances));
                self.instruments
                    .update_from_account_snapshots(&snapshot.instruments);
            }
            AccountEventKind::BalanceSnapshot(balance) => {
                self.assets.update_from_balance(balance.as_ref());
            }
            AccountEventKind::OrderSnapshot(order) => {
                self.instruments.update_from_order_snapshot(order.as_ref())
            }
            AccountEventKind::PositionSnapshot(position) => {
                self.instruments
                    .update_from_position_snapshot(position.as_ref());
            }
            AccountEventKind::OrderOpened(response) => self.instruments.update_from_open(response),
            AccountEventKind::OrderCancelled(response) => {
                self.instruments.update_from_cancel(response)
            }
            AccountEventKind::Trade(trade) => {
                self.instruments.update_from_trade(trade);
            }
            AccountEventKind::ConnectivityError(error) => {
                warn!(%error, %exchange, "Engine State aware of Account ConnectivityError");
            }
        }

        // Update any user provided Strategy & Risk State
        self.strategy.update(event);
        self.risk.update(event);
    }
}
