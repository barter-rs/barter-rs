use barter::{
    engine::state::{instrument::manager::InstrumentFilter, EngineState},
    strategy::{algo::AlgoStrategy, close_positions::ClosePositionsStrategy, DefaultStrategy},
};
use barter_execution::order::{Order, RequestCancel, RequestOpen};
use barter_instrument::{asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex};
use indexmap::IndexSet;

// MultiStrategy
// Each Strategy uses that same State (ie/ EngineState<...>).
// Each Instrument is associated with one of the Strategies enum variants.
pub struct MultiStrategy<State> {
    pub instrument_strategies: IndexSet<Strategies<State>>,
}

// Define enum containing all possible strategies. Each variant should implement the Strategy
// traits.
//
// For example purposes, DefaultStrategy is re-used (but you'd unique variants of your choosing)
pub enum Strategies<State> {
    Dumb(DefaultStrategy<State>),
    Dumber(DefaultStrategy<State>),
}

// Define enum containing all possible "on disconnect" outputs (one for each Strategy).
//
// For example purposes, DefaultStrategy is re-used, so it's just () for both.
pub enum StrategiesOnDisconnect {
    Dumb(()),
    Dumber(()),
}

// Define enum containing all possible "on disconnect" outputs (one for each Strategy).
//
// For example purposes, DefaultStrategy is re-used, so it's just () for both.
pub enum StrategiesOnTradingDisabled {
    Dumb(()),
    Dumber(()),
}

impl<State> AlgoStrategy for MultiStrategy<State> {
    type State = State;

    fn generate_algo_orders(
        &self,
        state: &Self::State,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeIndex, InstrumentIndex, RequestCancel>>,
        impl IntoIterator<Item = Order<ExchangeIndex, InstrumentIndex, RequestOpen>>,
    ) {
        self.instrument_strategies
            .iter()
            .map(|strategy| match strategy {
                Strategies::Dumb(dumb) => dumb.generate_algo_orders(state),
                Strategies::Dumber(dumber) => dumber.generate_algo_orders(state),
            })
            .fold(
                (Vec::new(), Vec::new()),
                |(mut agg_cancels, mut agg_opens), (cancels, opens)| {
                    agg_cancels.extend(cancels);
                    agg_opens.extend(opens);
                    (agg_cancels, agg_opens)
                },
            )
    }
}

impl<MarketState, StrategyState, RiskState> ClosePositionsStrategy
    for MultiStrategy<EngineState<MarketState, StrategyState, RiskState>>
{
    type State = EngineState<MarketState, StrategyState, RiskState>;

    fn close_positions_requests<'a>(
        &'a self,
        state: &'a Self::State,
        filter: &'a InstrumentFilter<ExchangeIndex, AssetIndex, InstrumentIndex>,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeIndex, InstrumentIndex, RequestCancel>> + 'a,
        impl IntoIterator<Item = Order<ExchangeIndex, InstrumentIndex, RequestOpen>> + 'a,
    )
    where
        ExchangeIndex: 'a,
        AssetIndex: 'a,
        InstrumentIndex: 'a,
    {
        self.instrument_strategies
            .iter()
            .map(|strategy| match strategy {
                Strategies::Dumb(dumb) => dumb.close_positions_requests(state, filter),
                Strategies::Dumber(dumber) => dumber.close_positions_requests(state, filter),
            })
            .fold(
                (Vec::new(), Vec::new()),
                |(mut agg_cancels, mut agg_opens), (cancels, opens)| {
                    agg_cancels.extend(cancels);
                    agg_opens.extend(opens);
                    (agg_cancels, agg_opens)
                },
            )
    }
}

// Todo: Fix
// impl<State, ExecutionTxs, Risk> OnDisconnectStrategy<State, ExecutionTxs, Risk> for MultiStrategy<State> {
//     type OnDisconnect = Vec<StrategiesOnDisconnect>;
//
//     fn on_disconnect(
//         engine: &mut Engine<State, ExecutionTxs, Self, Risk>,
//         exchange: ExchangeId,
//     ) -> Self::OnDisconnect {
//
//         engine
//             .strategy
//             .instrument_strategies
//             .iter()
//             .map(|strategy| match strategy {
//                 Strategies::Dumb(_) => {
//                     StrategiesOnDisconnect::Dumb(DefaultStrategy::on_disconnect(engine, exchange))
//                 },
//                 Strategies::Dumber(_) => {
//                     StrategiesOnDisconnect::Dumber(DefaultStrategy::on_disconnect(engine, exchange))
//                 },
//             })
//             .collect::<Vec<_>>()
//     }
// }
//
// impl<State, ExecutionTxs, Risk> OnTradingDisabled<State, ExecutionTxs, Risk> for MultiStrategy<State>
// {
//     type OnTradingDisabled = Vec<StrategiesOnTradingDisabled>;
//
//     fn on_trading_disabled(
//         engine: &mut Engine<State, ExecutionTxs, Self, Risk>,
//     ) -> Self::OnTradingDisabled {
//         engine
//             .strategy
//             .instrument_strategies
//             .iter()
//             .map(|strategy| match strategy {
//                 Strategies::Dumb(_) => {
//                     StrategiesOnTradingDisabled::Dumb(DefaultStrategy::on_trading_disabled(engine))
//                 },
//                 Strategies::Dumber(_) => {
//                     StrategiesOnTradingDisabled::Dumber(DefaultStrategy::on_trading_disabled(engine))
//                 },
//             })
//             .collect::<Vec<_>>()
//     }
// }

fn main() {}
