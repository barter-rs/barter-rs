use crate::v2::{
    engine::state::{
        instrument::market_data::MarketDataState,
        order::{manager::OrderManager, Orders},
    },
    execution::InstrumentAccountSnapshot,
    position::{Position, PositionExited},
    trade::Trade,
    Snapshot,
};
use barter_data::event::MarketEvent;
use barter_instrument::instrument::{name::InstrumentNameInternal, Instrument};
use derive_more::Constructor;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub mod manager;
pub mod market_data;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct InstrumentStates<Market, ExchangeKey, AssetKey, InstrumentKey>(
    pub  IndexMap<
        InstrumentNameInternal,
        InstrumentState<Market, ExchangeKey, AssetKey, InstrumentKey>,
    >,
);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct InstrumentState<Market, ExchangeKey, AssetKey, InstrumentKey> {
    pub key: InstrumentKey,
    pub instrument: Instrument<ExchangeKey, AssetKey>,
    pub position: Option<Position<AssetKey, InstrumentKey>>,
    pub orders: Orders<ExchangeKey, InstrumentKey>,
    pub market: Market,
}

impl<Market, ExchangeKey, AssetKey, InstrumentKey>
    InstrumentState<Market, ExchangeKey, AssetKey, InstrumentKey>
{
    pub fn update_from_account_snapshot(
        &mut self,
        snapshot: &InstrumentAccountSnapshot<ExchangeKey, InstrumentKey>,
    ) where
        ExchangeKey: Debug + Clone,
        InstrumentKey: Debug + Clone,
        AssetKey: Clone,
    {
        let InstrumentAccountSnapshot {
            instrument: _,
            orders,
        } = snapshot;

        for order in orders {
            self.orders.update_from_order_snapshot(Snapshot(order))
        }
    }

    pub fn update_from_trade(
        &mut self,
        trade: &Trade<AssetKey, InstrumentKey>,
    ) -> Option<PositionExited<AssetKey, InstrumentKey>>
    where
        AssetKey: Debug + Clone + PartialEq,
        InstrumentKey: Debug + Clone + PartialEq,
    {
        let (current, closed) = match self.position.take() {
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

        self.position = current;
        closed
    }

    pub fn update_from_market(&mut self, event: &MarketEvent<InstrumentKey, Market::EventKind>)
    where
        Market: MarketDataState<InstrumentKey>,
    {
        self.market.process(event);

        let Some(position) = &mut self.position else {
            return;
        };

        let Some(price) = self.market.price() else {
            return;
        };

        position.update_pnl_unrealised(price);
    }
}

#[cfg(test)]
mod tests {}
