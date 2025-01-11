use crate::{
    engine::state::{
        instrument::{filter::InstrumentFilter, market_data::MarketDataState},
        order::{manager::OrderManager, Orders},
        position::{PositionExited, PositionManager},
    },
    statistic::summary::instrument::TearSheetGenerator,
};
use barter_data::event::MarketEvent;
use barter_execution::{
    order::{
        request::OrderResponseCancel,
        state::{ActiveOrderState, OrderState},
        Order, OrderKey,
    },
    trade::Trade,
    InstrumentAccountSnapshot,
};
use barter_instrument::{
    asset::{name::AssetNameExchange, AssetIndex, QuoteAsset},
    exchange::{ExchangeId, ExchangeIndex},
    index::IndexedInstruments,
    instrument::{
        name::{InstrumentNameExchange, InstrumentNameInternal},
        Instrument, InstrumentIndex,
    },
};
use barter_integration::{collection::FnvIndexMap, snapshot::Snapshot};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use itertools::Either;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Defines the instrument-centric [`MarketDataState`] interface.
pub mod market_data;

/// Defines an `InstrumentFilter`, used to filter instrument-centric data structures.
pub mod filter;

/// Collection of [`InstrumentState`]s indexed by [`InstrumentIndex`].
///
/// Note that the same instruments with the same [`InstrumentNameExchange`] (eg/ "btc_usdt") but
/// on different exchanges will have their own [`InstrumentState`].
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct InstrumentStates<
    Market,
    ExchangeKey = ExchangeIndex,
    AssetKey = AssetIndex,
    InstrumentKey = InstrumentIndex,
>(
    pub  FnvIndexMap<
        InstrumentNameInternal,
        InstrumentState<Market, ExchangeKey, AssetKey, InstrumentKey>,
    >,
);

impl<Market> InstrumentStates<Market> {
    /// Return a reference to the `InstrumentState` associated with an `InstrumentIndex`.
    ///
    /// Panics if `InstrumentState` associated with the `InstrumentIndex` does not exist.
    pub fn instrument_index(&self, key: &InstrumentIndex) -> &InstrumentState<Market> {
        self.0
            .get_index(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("InstrumentStates does not contain: {key}"))
    }

    /// Return a mutable reference to the `InstrumentState` associated with an `InstrumentIndex`.
    ///
    /// Panics if `InstrumentState` associated with the `InstrumentIndex` does not exist.
    pub fn instrument_index_mut(&mut self, key: &InstrumentIndex) -> &mut InstrumentState<Market> {
        self.0
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("InstrumentStates does not contain: {key}"))
    }

    /// Return a reference to the `InstrumentState` associated with an `InstrumentNameInternal`.
    ///
    /// Panics if `InstrumentState` associated with the `InstrumentNameInternal` does not exist.
    pub fn instrument(&self, key: &InstrumentNameInternal) -> &InstrumentState<Market> {
        self.0
            .get(key)
            .unwrap_or_else(|| panic!("InstrumentStates does not contain: {key}"))
    }

    /// Return a mutable reference to the `InstrumentState` associated with an
    /// `InstrumentNameInternal`.
    ///
    /// Panics if `InstrumentState` associated with the `InstrumentNameInternal` does not exist.
    pub fn instrument_mut(&mut self, key: &InstrumentNameInternal) -> &mut InstrumentState<Market> {
        self.0
            .get_mut(key)
            .unwrap_or_else(|| panic!("InstrumentStates does not contain: {key}"))
    }

    /// Return an `Iterator` of all `InstrumentState`s being tracked, optionally filtered by the
    /// provided `InstrumentFilter`.
    pub fn instruments<'a>(
        &'a self,
        filter: &'a InstrumentFilter,
    ) -> impl Iterator<Item = &'a InstrumentState<Market>> {
        self.filtered(filter)
    }

    /// Return a filtered `Iterator` of `InstrumentState`s based on the provided `InstrumentFilter`.
    fn filtered<'a>(
        &'a self,
        filter: &'a InstrumentFilter,
    ) -> impl Iterator<Item = &'a InstrumentState<Market>>
    where
        Market: 'a,
    {
        use filter::InstrumentFilter::*;
        match filter {
            None => Either::Left(Either::Left(self.0.values())),
            Exchanges(exchanges) => Either::Left(Either::Right(
                self.0
                    .values()
                    .filter(|state| exchanges.contains(&state.instrument.exchange)),
            )),
            Instruments(instruments) => Either::Right(Either::Right(
                self.0
                    .values()
                    .filter(|state| instruments.contains(&state.key)),
            )),
            Underlyings(underlying) => Either::Right(Either::Left(
                self.0
                    .values()
                    .filter(|state| underlying.contains(&state.instrument.underlying)),
            )),
        }
    }

    /// Return an `Iterator` of instrument `TearSheetGenerator`s, optionally filtered by the
    /// provided `InstrumentFilter`.
    pub fn tear_sheets<'a>(
        &'a self,
        filter: &'a InstrumentFilter,
    ) -> impl Iterator<Item = &'a TearSheetGenerator>
    where
        Market: 'a,
    {
        self.filtered(filter).map(|state| &state.tear_sheet)
    }

    /// Return an `Iterator` of instrument `PositionManager`s, optionally filtered by the
    /// provided `InstrumentFilter`.
    pub fn positions<'a>(
        &'a self,
        filter: &'a InstrumentFilter,
    ) -> impl Iterator<Item = &'a PositionManager>
    where
        Market: 'a,
    {
        self.filtered(filter).map(|state| &state.position)
    }

    /// Return an `Iterator` of all instrument `Orders`s, optionally filtered by the
    /// provided `InstrumentFilter`.
    pub fn orders<'a>(&'a self, filter: &'a InstrumentFilter) -> impl Iterator<Item = &'a Orders>
    where
        Market: 'a,
    {
        self.filtered(filter).map(|state| &state.orders)
    }

    /// Return an `Iterator` of all instrument `MarketDataState`s, optionally filtered by the
    /// provided `InstrumentFilter`.
    pub fn market_datas<'a>(
        &'a self,
        filter: &'a InstrumentFilter,
    ) -> impl Iterator<Item = &'a Market>
    where
        Market: 'a,
    {
        self.filtered(filter).map(|state| &state.market)
    }
}

/// Represents the current state of an instrument, including its [`Position`], [`Orders`], and
/// user provided market data state.
///
/// This aggregates all the critical trading state for a single instrument, providing a complete
/// view of its current trading status and market conditions.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct InstrumentState<
    Market,
    ExchangeKey = ExchangeIndex,
    AssetKey = AssetIndex,
    InstrumentKey = InstrumentIndex,
> {
    /// Unique `InstrumentKey` identifier for the instrument this state is associated with.
    pub key: InstrumentKey,

    /// Complete instrument definition.
    pub instrument: Instrument<ExchangeKey, AssetKey>,

    /// TearSheet generator for summarising the trading performance associated with an Instrument.
    pub tear_sheet: TearSheetGenerator,

    /// Current `PositionManager`.
    pub position: PositionManager<InstrumentKey>,

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
        snapshot: &InstrumentAccountSnapshot<ExchangeKey, AssetKey, InstrumentKey>,
    ) where
        ExchangeKey: Debug + Clone,
        InstrumentKey: Debug + Clone,
        AssetKey: Debug + Clone,
    {
        for order in &snapshot.orders {
            self.update_from_order_snapshot(Snapshot(order))
        }
    }

    /// Updates the instrument state from an [`Order`] snapshot.
    pub fn update_from_order_snapshot(
        &mut self,
        order: Snapshot<&Order<ExchangeKey, InstrumentKey, OrderState<AssetKey, InstrumentKey>>>,
    ) where
        ExchangeKey: Debug + Clone,
        AssetKey: Debug + Clone,
        InstrumentKey: Debug + Clone,
    {
        self.orders.update_from_order_snapshot(order);
    }

    /// Updates the instrument state from an
    /// [`OrderRequestCancel`](barter_execution::order::request::OrderRequestCancel) response.
    pub fn update_from_cancel_response(
        &mut self,
        response: &OrderResponseCancel<ExchangeKey, AssetKey, InstrumentKey>,
    ) where
        ExchangeKey: Debug + Clone,
        AssetKey: Debug + Clone,
        InstrumentKey: Debug + Clone,
    {
        self.orders
            .update_from_cancel_response::<AssetKey>(response);
    }

    /// Updates the instrument state based on a new trade.
    ///
    /// This method handles:
    /// - Opening/updating the current position state based on a new trade.
    /// - Updating the internal [`TearSheetGenerator`] if a position is exited.
    pub fn update_from_trade(
        &mut self,
        trade: &Trade<QuoteAsset, InstrumentKey>,
    ) -> Option<PositionExited<QuoteAsset, InstrumentKey>>
    where
        InstrumentKey: Debug + Clone + PartialEq,
    {
        self.position
            .update_from_trade(trade)
            .inspect(|closed| self.tear_sheet.update_from_position(closed))
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

        let Some(position) = &mut self.position.current else {
            return;
        };

        let Some(price) = self.market.price() else {
            return;
        };

        position.update_pnl_unrealised(price);
    }
}

pub fn generate_unindexed_instrument_account_snapshot<
    Market,
    ExchangeKey,
    AssetKey,
    InstrumentKey,
>(
    exchange: ExchangeId,
    state: &InstrumentState<Market, ExchangeKey, AssetKey, InstrumentKey>,
) -> InstrumentAccountSnapshot<ExchangeId, AssetNameExchange, InstrumentNameExchange>
where
    ExchangeKey: Debug + Clone,
    InstrumentKey: Debug + Clone,
{
    let InstrumentState {
        key: _,
        instrument,
        tear_sheet: _,
        position: _,
        orders,
        market: _,
    } = state;

    InstrumentAccountSnapshot {
        instrument: instrument.name_exchange.clone(),
        orders: orders
            .orders()
            .filter_map(|order| {
                let Order {
                    key,
                    side,
                    price,
                    quantity,
                    kind,
                    time_in_force,
                    state: ActiveOrderState::Open(open),
                } = order
                else {
                    return None;
                };

                Some(Order {
                    key: OrderKey {
                        exchange,
                        instrument: instrument.name_exchange.clone(),
                        strategy: key.strategy.clone(),
                        cid: key.cid.clone(),
                    },
                    side: *side,
                    price: *price,
                    quantity: *quantity,
                    kind: *kind,
                    time_in_force: *time_in_force,
                    state: OrderState::active(open.clone()),
                })
            })
            .collect(),
    }
}

/// Generates an indexed [`InstrumentStates`] containing default instrument state data.
pub fn generate_empty_indexed_instrument_states<Market>(
    instruments: &IndexedInstruments,
    time_engine_start: DateTime<Utc>,
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
                        TearSheetGenerator::init(time_engine_start),
                        PositionManager::default(),
                        Orders::default(),
                        Market::default(),
                    ),
                )
            })
            .collect(),
    )
}
