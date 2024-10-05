use crate::v2::engine::state::instrument::Instruments;
use crate::v2::position::{PortfolioId, Position};
use crate::v2::trade::Trade;
use crate::v2::Snapshot;
use std::fmt::Debug;
use std::hash::Hash;
use tracing::warn;

pub trait PositionManager<InstrumentKey> {
    fn update_from_position_snapshot(&mut self, snapshot: Snapshot<&Position<InstrumentKey>>);
    fn position(&self, instrument: &InstrumentKey) -> Option<&Position<InstrumentKey>>;
    fn positions_by_portfolio<'a>(
        &'a self,
        portfolio: PortfolioId,
    ) -> impl Iterator<Item = &'a Position<InstrumentKey>>
    where
        InstrumentKey: 'a;
    fn update_from_trade(&mut self, trade: &Trade<InstrumentKey>);
}

impl<InstrumentKey, MarketState> PositionManager<InstrumentKey>
    for Instruments<InstrumentKey, MarketState>
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

    fn update_from_trade(&mut self, _trade: &Trade<InstrumentKey>) {
        // Todo: should Trade contain PortfolioId? Or could remove concept for now...
        todo!()
    }
}
