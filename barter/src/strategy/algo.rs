use barter_execution::order::request::{OrderRequestCancel, OrderRequestOpen};
use barter_instrument::{exchange::ExchangeIndex, instrument::InstrumentIndex};

/// Strategy interface for generating algorithmic open and cancel order requests based on the
/// current `EngineState`.
///
/// # Type Parameters
/// * `ExchangeKey` - Type used to identify an exchange (defaults to [`ExchangeIndex`]).
/// * `InstrumentKey` - Type used to identify an instrument (defaults to [`InstrumentIndex`]).
pub trait AlgoStrategy<ExchangeKey = ExchangeIndex, InstrumentKey = InstrumentIndex> {
    /// State used by the `AlgoStrategy` to determine what open and cancel requests to generate.
    ///
    /// For Barter ecosystem strategies, this is the full `EngineState` of the trading system.
    ///
    /// eg/ `EngineState<DefaultGlobalData, DefaultInstrumentMarketData>`
    type State;

    /// Generate algorithmic orders based on current system `State`.
    fn generate_algo_orders(
        &self,
        state: &Self::State,
    ) -> (
        impl IntoIterator<Item = OrderRequestCancel<ExchangeKey, InstrumentKey>>,
        impl IntoIterator<Item = OrderRequestOpen<ExchangeKey, InstrumentKey>>,
    );
}
