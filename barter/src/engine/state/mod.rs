use crate::engine::{
    state::{
        asset::{generate_empty_indexed_asset_states, AssetStates},
        connectivity::{generate_empty_indexed_connectivity_states, ConnectivityStates},
        instrument::{
            generate_empty_indexed_instrument_states, market_data::MarketDataState,
            InstrumentStates,
        },
        order::manager::OrderManager,
        position::PositionExited,
        trading::TradingState,
    },
    Processor,
};
use barter_data::event::MarketEvent;
use barter_execution::{AccountEvent, AccountEventKind};
use barter_instrument::{
    asset::{AssetIndex, QuoteAsset},
    exchange::ExchangeIndex,
    index::IndexedInstruments,
    instrument::InstrumentIndex,
};
use barter_integration::snapshot::Snapshot;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub mod asset;
pub mod connectivity;
pub mod instrument;
pub mod order;
pub mod position;
pub mod trading;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EngineState<Market, Strategy, Risk> {
    pub trading: TradingState,
    pub connectivity: ConnectivityStates,
    pub assets: AssetStates,
    pub instruments: InstrumentStates<Market, ExchangeIndex, AssetIndex, InstrumentIndex>,
    pub strategy: Strategy,
    pub risk: Risk,
}

impl<Market, Strategy, Risk> EngineState<Market, Strategy, Risk> {
    pub fn update_from_account(
        &mut self,
        event: &AccountEvent,
    ) -> Option<PositionExited<QuoteAsset>>
    where
        Strategy: for<'a> Processor<&'a AccountEvent>,
        Risk: for<'a> Processor<&'a AccountEvent>,
    {
        // Set exchange account connectivity to Healthy if it was Reconnecting
        self.connectivity.update_from_account_event(&event.exchange);

        let output = match &event.kind {
            AccountEventKind::Snapshot(snapshot) => {
                for balance in &snapshot.balances {
                    self.assets
                        .asset_index_mut(&balance.asset)
                        .update_from_balance(Snapshot(balance))
                }
                for instrument in &snapshot.instruments {
                    self.instruments
                        .instrument_index_mut(&instrument.instrument)
                        .update_from_account_snapshot(instrument)
                }
                None
            }
            AccountEventKind::BalanceSnapshot(balance) => {
                self.assets
                    .asset_index_mut(&balance.0.asset)
                    .update_from_balance(balance.as_ref());
                None
            }
            AccountEventKind::OrderSnapshot(order) => {
                self.instruments
                    .instrument_index_mut(&order.0.instrument)
                    .orders
                    .update_from_order_snapshot(order.as_ref());
                None
            }
            AccountEventKind::OrderOpened(response) => {
                self.instruments
                    .instrument_index_mut(&response.instrument)
                    .orders
                    .update_from_open(response);
                None
            }
            AccountEventKind::OrderCancelled(response) => {
                self.instruments
                    .instrument_index_mut(&response.instrument)
                    .orders
                    .update_from_cancel(response);
                None
            }
            AccountEventKind::Trade(trade) => self
                .instruments
                .instrument_index_mut(&trade.instrument)
                .update_from_trade(trade),
        };

        // Update any user provided Strategy & Risk State
        self.strategy.process(event);
        self.risk.process(event);

        output
    }

    pub fn update_from_market(&mut self, event: &MarketEvent<InstrumentIndex, Market::EventKind>)
    where
        Market: MarketDataState,
        Strategy: for<'a> Processor<&'a MarketEvent<InstrumentIndex, Market::EventKind>>,
        Risk: for<'a> Processor<&'a MarketEvent<InstrumentIndex, Market::EventKind>>,
    {
        // Set exchange market data connectivity to Healthy if it was Reconnecting
        self.connectivity.update_from_market_event(&event.exchange);

        let instrument_state = self.instruments.instrument_index_mut(&event.instrument);

        instrument_state.market.process(event);
        self.strategy.process(event);
        self.risk.process(event);
    }
}

pub fn generate_empty_indexed_engine_state<Market, Strategy, Risk>(
    trading_state: TradingState,
    instruments: &IndexedInstruments,
    strategy: Strategy,
    risk: Risk,
) -> EngineState<Market, Strategy, Risk>
where
    Market: Default,
{
    EngineState {
        trading: trading_state,
        connectivity: generate_empty_indexed_connectivity_states(instruments),
        assets: generate_empty_indexed_asset_states(instruments),
        instruments: generate_empty_indexed_instrument_states::<Market>(instruments),
        strategy,
        risk,
    }
}
