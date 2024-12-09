use crate::{
    engine::state::{
        instrument::market_data::MarketDataState,
        order::{manager::OrderManager, Orders},
        position::{Position, PositionExited},
    },
    FnvIndexMap,
};
use barter_data::event::MarketEvent;
use barter_execution::{trade::Trade, InstrumentAccountSnapshot};
use barter_instrument::{
    asset::{AssetIndex, QuoteAsset},
    exchange::ExchangeIndex,
    index::IndexedInstruments,
    instrument::{name::InstrumentNameInternal, Instrument, InstrumentIndex},
};
use barter_integration::snapshot::Snapshot;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub mod manager;
pub mod market_data;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct InstrumentStates<Market, ExchangeKey, AssetKey, InstrumentKey>(
    pub  FnvIndexMap<
        InstrumentNameInternal,
        InstrumentState<Market, ExchangeKey, AssetKey, InstrumentKey>,
    >,
);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct InstrumentState<Market, ExchangeKey, AssetKey, InstrumentKey> {
    pub key: InstrumentKey,
    pub instrument: Instrument<ExchangeKey, AssetKey>,
    pub position: Option<Position<QuoteAsset, InstrumentKey>>,
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
        for order in &snapshot.orders {
            self.orders.update_from_order_snapshot(Snapshot(order))
        }
    }

    pub fn update_from_trade(
        &mut self,
        trade: &Trade<QuoteAsset, InstrumentKey>,
    ) -> Option<PositionExited<QuoteAsset, InstrumentKey>>
    where
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

pub fn generate_default_instrument_states<Market>(
    instruments: &IndexedInstruments,
) -> InstrumentStates<Market, ExchangeIndex, AssetIndex, InstrumentIndex>
where
    Market: Default,
{
    InstrumentStates(
        instruments
            .instruments
            .iter()
            .map(|instrument| {
                let exchange_index = instrument.value.exchange.key;
                (
                    instrument.value.name_internal.clone(),
                    InstrumentState::new(
                        instrument.key,
                        instrument.value.clone().map_exchange_key(exchange_index),
                        None,
                        Orders::default(),
                        Market::default(),
                    ),
                )
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {}
