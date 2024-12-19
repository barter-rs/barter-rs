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

/// Collection of [`InstrumentState`]s indexed by [`InstrumentIndex`].
///
/// Note that the same instruments with the same [`InstrumentNameExchange`] (eg/ "btc_usdt") but
/// on different exchanges will have their own [`InstrumentState`].
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct InstrumentStates<Market, ExchangeKey, AssetKey, InstrumentKey>(
    pub  FnvIndexMap<
        InstrumentNameInternal,
        InstrumentState<Market, ExchangeKey, AssetKey, InstrumentKey>,
    >,
);

/// Represents the current state of an instrument, including its [`Position`], [`Orders`], and
/// user provided market data state.
///
/// This aggregates all the critical trading state for a single instrument, providing a complete
/// view of its current trading status and market conditions.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct InstrumentState<Market, ExchangeKey, AssetKey, InstrumentKey> {
    /// Unique `InstrumentKey` identifier for the instrument this state is associated with.
    pub key: InstrumentKey,

    /// Complete instrument definition.
    pub instrument: Instrument<ExchangeKey, AssetKey>,

    /// Current open position.
    pub position: Option<Position<QuoteAsset, InstrumentKey>>,

    /// Active orders and associated order management.
    pub orders: Orders<ExchangeKey, InstrumentKey>,

    /// User provided market data state associated with this instrument.
    pub market: Market,
}

impl<Market, ExchangeKey, AssetKey, InstrumentKey>
    InstrumentState<Market, ExchangeKey, AssetKey, InstrumentKey>
{
    /// Updates the instrument state using an account snapshot from the exchange.
    ///
    /// This updates active orders for the instrument, using timestamps where relevant to ensure
    /// the most recent order state is applied.
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

    /// Updates the instrument's position state based on a new trade.
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

    /// Updates the instrument's market data state from a new market event.
    ///
    /// If the market event has a price associated with it (eg/ `PublicTrade`, `OrderBookL1`), any
    /// open [`Position`] `pnl_unrealised` is re-calculated.
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

/// Generates an indexed [`InstrumentStates`] containing default instrument state data.
pub fn generate_empty_indexed_instrument_states<Market>(
    instruments: &IndexedInstruments,
) -> InstrumentStates<Market, ExchangeIndex, AssetIndex, InstrumentIndex>
where
    Market: Default,
{
    InstrumentStates(
        instruments
            .instruments()
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
