pub mod asset;
pub mod instrument;
mod order_manager;
pub mod processor;
pub mod trading;

use crate::v2::{
    engine_new::state::{asset::AssetStates, instrument::InstrumentStates, trading::TradingState},
    execution::{error::ConnectivityError, AccountSnapshot},
};
use barter_instrument::{asset::AssetIndex, exchange::ExchangeId, instrument::InstrumentIndex};
use vecmap::VecMap;
// Todo:
//  - Consider introducing AssetKey, InstrumentKey, to State, with two impls for Id vs Index,
//    that way I can go hard-core on the Indexes, but it's opt in at a high level
//  - Maybe introduce State machine for dealing with connectivity VecMap issue...
//    '--> could only check if a new Account/Market event updates to Connected if we are in
//         State=Unhealthy, that way we are only doing expensive lookup in that case
//  - Need to make some Key decisions about "what is a manager", and "what is an Updater"

pub trait Updater<Event> {
    type Output;
    fn update(&mut self, event: &Event) -> Self::Output;
}

pub struct EngineState<Market, Strategy, Risk> {
    pub connectivity: VecMap<ExchangeId, ConnectivityState>,
    pub trading: TradingState,
    pub instruments: InstrumentStates<Market>,
    pub assets: AssetStates,
    pub strategy: Strategy,
    pub risk: Risk,
}

impl<Market, Strategy, Risk> EngineState<Market, Strategy, Risk> {
    pub fn update_from_account_snapshot(
        &mut self,
        snapshot: AccountSnapshot<AssetIndex, InstrumentIndex>,
    ) {
        let AccountSnapshot {
            balances,
            instruments,
        } = snapshot;
    }

    // pub fn instrument_by_index(&self, instrument: InstrumentIndex) -> &InstrumentState<Market> {
    //     self.instruments.state_by_index(instrument)
    // }
    //
    // pub fn instrument_by_index_mut(&mut self, instrument: InstrumentIndex) -> &mut InstrumentState<Market> {
    //     self.instruments.state_by_index_mut(instrument)
    // }
    //
    // pub fn asset_by_index(&self, asset: AssetIndex) -> &AssetState {
    //     self.assets.state_by_index(asset)
    // }
    //
    // pub fn asset_by_index_mut(&mut self, asset: AssetIndex) -> &mut AssetState {
    //     self.assets.state_by_index_mut(asset)
    // }
}

pub struct ConnectivityState {
    market_data: Connection,
    account: Connection,
}

pub enum Connection {
    Healthy,
    Unhealthy(ConnectivityError),
    Reconnecting,
}
