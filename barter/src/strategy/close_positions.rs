use crate::engine::state::{instrument::filter::InstrumentFilter, EngineState};
use barter_execution::order::{
    ClientOrderId, Order, OrderKind, RequestCancel, RequestOpen, StrategyId, TimeInForce,
};
use barter_instrument::{
    asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex, Side,
};

pub trait ClosePositionsStrategy<
    ExchangeKey = ExchangeIndex,
    AssetKey = AssetIndex,
    InstrumentKey = InstrumentIndex,
>
{
    type State;

    fn close_positions_requests<'a>(
        &'a self,
        state: &'a Self::State,
        filter: &'a InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>> + 'a,
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>> + 'a,
    )
    where
        ExchangeKey: 'a,
        AssetKey: 'a,
        InstrumentKey: 'a;
}

pub fn close_open_positions_with_market_orders<'a, MarketState, StrategyState, RiskState>(
    strategy_id: &'a StrategyId,
    state: &'a EngineState<MarketState, StrategyState, RiskState>,
    filter: &'a InstrumentFilter,
) -> (
    impl IntoIterator<Item = Order<ExchangeIndex, InstrumentIndex, RequestCancel>> + 'a,
    impl IntoIterator<Item = Order<ExchangeIndex, InstrumentIndex, RequestOpen>> + 'a,
)
where
    ExchangeIndex: 'a,
    AssetIndex: 'a,
    InstrumentIndex: 'a,
{
    let open_requests = state.instruments.filtered(filter).filter_map(|state| {
        let position = state.position.as_ref()?;

        Some(Order {
            exchange: state.instrument.exchange,
            instrument: position.instrument,
            strategy: strategy_id.clone(),
            cid: ClientOrderId::default(),
            side: match position.side {
                Side::Buy => Side::Sell,
                Side::Sell => Side::Buy,
            },
            state: RequestOpen {
                kind: OrderKind::Market,
                time_in_force: TimeInForce::ImmediateOrCancel,
                price: Default::default(),
                quantity: position.quantity_abs,
            },
        })
    });

    (std::iter::empty(), open_requests)
}
