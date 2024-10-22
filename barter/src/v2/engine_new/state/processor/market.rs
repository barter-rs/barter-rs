use crate::v2::engine_new::state::{EngineState, Updater};
use barter_data::event::MarketEvent;
use barter_instrument::instrument::InstrumentIndex;

impl<Market, Strategy, Risk, Kind> Updater<MarketEvent<InstrumentIndex, Kind>>
    for EngineState<Market, Strategy, Risk>
{
    type Output = ();

    fn update(&mut self, event: &MarketEvent<InstrumentIndex, Kind>) -> Self::Output {
        todo!()
    }
}
