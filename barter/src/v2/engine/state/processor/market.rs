use crate::v2::engine::state::{EngineState, Updater};
use barter_data::event::{DataKind, MarketEvent};
use barter_instrument::instrument::InstrumentIndex;

impl<Kind, AssetKey, InstrumentKey, Market, Strategy, Risk> Updater<MarketEvent<InstrumentIndex, Kind>>
    for EngineState<AssetKey, InstrumentKey, Market, Strategy, Risk>
where
    Market: Updater<MarketEvent<InstrumentIndex, Kind>>,
    Strategy: Updater<MarketEvent<InstrumentIndex, Kind>>,
    Risk: Updater<MarketEvent<InstrumentIndex, Kind>>,
{
    type Output = ();

    fn update(&mut self, event: &MarketEvent<InstrumentIndex, Kind>) -> Self::Output {
        self.instruments.state_by_index_mut(event.instrument).market.update(event);
        self.risk.update(event);
        self.risk.update(event);
    }
}


