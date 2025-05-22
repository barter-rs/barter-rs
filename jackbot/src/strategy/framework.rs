use crate::strategy::{algo::AlgoStrategy, close_positions::ClosePositionsStrategy};
use jackbot_execution::order::id::StrategyId;
use jackbot_instrument::{asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex};

pub trait Strategy<E = ExchangeIndex, I = InstrumentIndex>:
    AlgoStrategy<E, I, State = Self::State> +
    ClosePositionsStrategy<E, AssetIndex, I, State = Self::State>
{
    type State;
    fn id(&self) -> StrategyId;
}
