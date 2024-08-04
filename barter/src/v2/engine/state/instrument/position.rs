use crate::v2::{
    engine::state::instrument::Instruments,
    position::{PortfolioId, Position},
    trade::Trade,
    Snapshot,
};
use std::{fmt::Debug, hash::Hash};
use tracing::warn;

pub trait PositionManager<AssetKey, InstrumentKey> {
    fn update_from_position_snapshot(&mut self, snapshot: Snapshot<&Position<InstrumentKey>>);
    fn position(&self, instrument: &InstrumentKey) -> Option<&Position<InstrumentKey>>;
    fn positions_by_portfolio<'a>(
        &'a self,
        portfolio: PortfolioId,
    ) -> impl Iterator<Item = &'a Position<InstrumentKey>>
    where
        InstrumentKey: 'a;
    fn update_from_trade(&mut self, trade: &Trade<AssetKey, InstrumentKey>);
}

impl<AssetKey, InstrumentKey, MarketState> PositionManager<AssetKey, InstrumentKey>
    for Instruments<AssetKey, InstrumentKey, MarketState>
where
    InstrumentKey: Debug + Clone + Eq + Hash,
{
    fn update_from_position_snapshot(&mut self, snapshot: Snapshot<&Position<InstrumentKey>>) {
        let Some(state) = self.state_mut(&snapshot.0.instrument) else {
            warn!(
                instrument_id = ?snapshot.0.instrument,
                event = ?snapshot,
                "OrderManager ignoring Snapshot<Position> received for non-configured instrument"
            );
            return;
        };

        state.position = snapshot.0.clone();
    }

    fn position(&self, instrument: &InstrumentKey) -> Option<&Position<InstrumentKey>> {
        self.state(instrument).map(|state| &state.position)
    }

    fn positions_by_portfolio<'a>(
        &'a self,
        portfolio: PortfolioId,
    ) -> impl Iterator<Item = &'a Position<InstrumentKey>>
    where
        InstrumentKey: 'a,
    {
        self.0.values().filter_map(move |state| {
            (state.position.portfolio == portfolio).then_some(&state.position)
        })
    }

    fn update_from_trade(&mut self, _trade: &Trade<AssetKey, InstrumentKey>) {
        // Todo: should Trade contain PortfolioId? Or could remove concept for now...
        todo!()
    }
}
