use crate::engine::state::{
    instrument::{filter::InstrumentFilter, market_data::MarketDataState},
    EngineState,
};
use barter_execution::order::{
    id::{ClientOrderId, StrategyId},
    request::{OrderRequestCancel, OrderRequestOpen, RequestOpen},
    OrderKey, OrderKind, TimeInForce,
};
use barter_instrument::{
    asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex, Side,
};

/// Strategy interface for generating open and cancel order requests that close open positions.
///
/// This allows full customisation of how a strategy will close a position.
///
/// Different strategies may:
/// - Use different order types (Market, Limit, etc.).
/// - Prioritise certain exchanges.
/// - Increase the position of an inversely correlated instrument in order to neutralise exposure.
/// - etc.
///
/// # Type Parameters
/// * `ExchangeKey` - Type used to identify an exchange (defaults to [`ExchangeIndex`]).
/// * `AssetKey` - Type used to identify an asset (defaults to [`AssetIndex`]).
/// * `InstrumentKey` - Type used to identify an instrument (defaults to [`InstrumentIndex`]).
pub trait ClosePositionsStrategy<
    ExchangeKey = ExchangeIndex,
    AssetKey = AssetIndex,
    InstrumentKey = InstrumentIndex,
>
{
    /// State used by the `ClosePositionsStrategy` to determine what open and cancel requests
    /// to generate.
    ///
    /// For Barter ecosystem strategies, this is the full `EngineState` of the trading system.
    ///
    /// eg/ `EngineState<DefaultMarketState, DefaultStrategyState, DefaultRiskManagerState>`
    type State;

    /// Generate orders based on current system `State`.
    fn close_positions_requests<'a>(
        &'a self,
        state: &'a Self::State,
        filter: &'a InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>,
    ) -> (
        impl IntoIterator<Item = OrderRequestCancel<ExchangeKey, InstrumentKey>> + 'a,
        impl IntoIterator<Item = OrderRequestOpen<ExchangeKey, InstrumentKey>> + 'a,
    )
    where
        ExchangeKey: 'a,
        AssetKey: 'a,
        InstrumentKey: 'a;
}

/// Naive `ClosePositionsStrategy` logic for closing open positions with market orders only.
///
/// This function finds all open positions and generates equal but opposite `Side` market orders
/// that will neutralise the position.
pub fn close_open_positions_with_market_orders<'a, MarketState, StrategyState, RiskState>(
    strategy_id: &'a StrategyId,
    state: &'a EngineState<MarketState, StrategyState, RiskState>,
    filter: &'a InstrumentFilter,
) -> (
    impl IntoIterator<Item = OrderRequestCancel<ExchangeIndex, InstrumentIndex>> + 'a,
    impl IntoIterator<Item = OrderRequestOpen<ExchangeIndex, InstrumentIndex>> + 'a,
)
where
    MarketState: MarketDataState,
    ExchangeIndex: 'a,
    AssetIndex: 'a,
    InstrumentIndex: 'a,
{
    let open_requests = state
        .instruments
        .instruments(filter)
        .filter_map(move |state| {
            // Only generate orders if there is a Position and we have market data
            let position = state.position.current.as_ref()?;
            let price = state.market.price()?;

            Some(OrderRequestOpen {
                key: OrderKey {
                    exchange: state.instrument.exchange,
                    instrument: position.instrument,
                    strategy: strategy_id.clone(),
                    cid: ClientOrderId::new(state.key.to_string()),
                },
                state: RequestOpen {
                    side: match position.side {
                        Side::Buy => Side::Sell,
                        Side::Sell => Side::Buy,
                    },
                    price,
                    quantity: position.quantity_abs,
                    kind: OrderKind::Market,
                    time_in_force: TimeInForce::ImmediateOrCancel,
                },
            })
        });

    (std::iter::empty(), open_requests)
}
