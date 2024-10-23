pub mod asset;
pub mod instrument;
mod order_manager;
pub mod processor;
pub mod trading;
mod connectivity;

use crate::v2::{
    engine::state::{asset::AssetStates, instrument::InstrumentStates, trading::TradingState},
    execution::{error::ConnectivityError, AccountSnapshot},
};
use barter_instrument::{asset::AssetIndex, exchange::ExchangeId, instrument::InstrumentIndex};
use vecmap::VecMap;
use crate::v2::engine::state::connectivity::ConnectivityState;
use crate::v2::engine::state::order_manager::OrderManager;
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

#[derive(Debug)]
pub struct EngineState<Market, Strategy, Risk> {
    pub connectivity: VecMap<ExchangeId, ConnectivityState>,
    pub trading: TradingState,
    pub instruments: InstrumentStates<Market>,
    pub assets: AssetStates,
    pub strategy: Strategy,
    pub risk: Risk,
}

impl<Market, Strategy, Risk> EngineState<Market, Strategy, Risk> {
    pub fn order_manager(&self) -> &impl OrderManager<InstrumentIndex> {
        &self.instruments
    }

    pub fn order_manager_mut(&mut self) -> &mut impl OrderManager<InstrumentIndex> {
        &mut self.instruments
    }
}


