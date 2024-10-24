pub mod asset;
pub mod instrument;
mod order_manager;
pub mod processor;
pub mod trading;
pub mod connectivity;

use crate::v2::{engine::state::{asset::AssetStates, instrument::InstrumentStates, trading::TradingState}, Snapshot};
use barter_instrument::{exchange::ExchangeId, instrument::InstrumentIndex};
use vecmap::VecMap;
use barter_instrument::asset::AssetIndex;
use crate::v2::engine::state::connectivity::ConnectivityState;
use crate::v2::engine::state::order_manager::OrderManager;
use crate::v2::execution::AccountSnapshot;
// Todo:
//  - Maybe introduce State machine for dealing with connectivity VecMap issue...
//    '--> could only check if a new Account/Market event updates to Connected if we are in
//         State=Unhealthy, that way we are only doing expensive lookup in that case
//  - Need to make some Key decisions about "what is a manager", and "what is an Updater"

// Todo: Consider splitting AccountEvents into AccountInstrumentEvents, AccountAssetEvent, Other
//       '--> ideally I can flip UPdate<AccountEvent> upside down to not duplicate logic
//       '--> issue becomes more impl Updater for user Strategy & Risk :(

pub trait Updater<Event> {
    type Output;
    fn update(&mut self, event: &Event) -> Self::Output;
}

pub trait StateManager<Key> {
    type State;
    fn state(&self, key: &Key) -> Option<&Self::State>;
    fn state_mut(&mut self, key: &Key) -> Option<&mut Self::State>;
}

#[derive(Debug)]
pub struct EngineState<AssetKey, InstrumentKey, Market, Strategy, Risk> {
    pub connectivity: VecMap<ExchangeId, ConnectivityState>,
    pub trading: TradingState,
    pub assets: AssetStates<AssetKey>,
    pub instruments: InstrumentStates<AssetKey, InstrumentKey, Market>,
    pub strategy: Strategy,
    pub risk: Risk,
}

impl<Market, Strategy, Risk> EngineState<AssetIndex, InstrumentIndex, Market, Strategy, Risk> {

}


