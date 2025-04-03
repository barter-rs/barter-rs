use crate::engine::state::{
    EngineState,
    instrument::{InstrumentState, data::InstrumentDataState, filter::InstrumentFilter},
    position::Position,
};
use barter_execution::order::{
    OrderKey, OrderKind, TimeInForce,
    id::{ClientOrderId, StrategyId},
    request::{OrderRequestCancel, OrderRequestOpen, RequestOpen},
};
use barter_instrument::{
    Side, asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex,
};
use rust_decimal::Decimal;

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
    /// eg/ `EngineState<DefaultGlobalData, DefaultInstrumentMarketData>`
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
pub fn close_open_positions_with_market_orders<'a, GlobalData, InstrumentData>(
    strategy_id: &'a StrategyId,
    state: &'a EngineState<GlobalData, InstrumentData>,
    filter: &'a InstrumentFilter,
    gen_cid: impl Fn(&InstrumentState<InstrumentData>) -> ClientOrderId + Copy + 'a,
) -> (
    impl IntoIterator<Item = OrderRequestCancel<ExchangeIndex, InstrumentIndex>> + 'a,
    impl IntoIterator<Item = OrderRequestOpen<ExchangeIndex, InstrumentIndex>> + 'a,
)
where
    InstrumentData: InstrumentDataState,
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
            let price = state.data.price()?;

            Some(build_ioc_market_order_to_close_position(
                state.instrument.exchange,
                position,
                strategy_id.clone(),
                price,
                || gen_cid(state),
            ))
        });

    (std::iter::empty(), open_requests)
}

/// Build an equal but opposite `Side` `ImmediateOrCancel` `Market` order that neutralises the
/// provided [`Position`].
///
/// For example, if [`Position`] is LONG by 100, build a market order request to sell 100.
pub fn build_ioc_market_order_to_close_position<ExchangeKey, AssetKey, InstrumentKey>(
    exchange: ExchangeKey,
    position: &Position<AssetKey, InstrumentKey>,
    strategy_id: StrategyId,
    price: Decimal,
    gen_cid: impl Fn() -> ClientOrderId,
) -> OrderRequestOpen<ExchangeKey, InstrumentKey>
where
    ExchangeKey: Clone,
    InstrumentKey: Clone,
{
    OrderRequestOpen {
        key: OrderKey {
            exchange: exchange.clone(),
            instrument: position.instrument.clone(),
            strategy: strategy_id,
            cid: gen_cid(),
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
    }
}
