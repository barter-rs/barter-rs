use crate::{
    engine::{
        state::instrument::manager::{InstrumentFilter, InstrumentStateManager},
        Engine, Processor,
    },
    strategy::{
        algo::AlgoStrategy, close_positions::ClosePositionsStrategy,
        on_disconnect::OnDisconnectStrategy, on_trading_disabled::OnTradingDisabled,
    },
};
use barter_data::event::MarketEvent;
use barter_execution::{
    order::{ClientOrderId, Order, OrderKind, RequestCancel, RequestOpen, StrategyId, TimeInForce},
    AccountEvent,
};
use barter_instrument::{exchange::ExchangeId, Side};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use std::marker::PhantomData;

pub mod algo;
pub mod close_positions;
pub mod on_disconnect;
pub mod on_trading_disabled;

#[derive(Debug, Clone)]
pub struct DefaultStrategy<State> {
    pub id: StrategyId,
    phantom: PhantomData<State>,
}

impl<State> Default for DefaultStrategy<State> {
    fn default() -> Self {
        Self {
            id: StrategyId::new("default"),
            phantom: PhantomData,
        }
    }
}

impl<State, ExchangeKey, InstrumentKey> AlgoStrategy<ExchangeKey, InstrumentKey>
    for DefaultStrategy<State>
{
    type State = State;

    fn generate_algo_orders(
        &self,
        _: &Self::State,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>>,
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>>,
    ) {
        (std::iter::empty(), std::iter::empty())
    }
}

impl<State, ExchangeKey, AssetKey, InstrumentKey>
    ClosePositionsStrategy<ExchangeKey, AssetKey, InstrumentKey> for DefaultStrategy<State>
where
    State: InstrumentStateManager<InstrumentKey, ExchangeKey = ExchangeKey, AssetKey = AssetKey>,
    ExchangeKey: PartialEq + Clone,
    AssetKey: PartialEq,
    InstrumentKey: PartialEq + Clone,
{
    type State = State;

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
        InstrumentKey: 'a,
    {
        let open_requests = state.instruments_filtered(filter).filter_map(|state| {
            let position = state.position.as_ref()?;

            Some(Order {
                exchange: state.instrument.exchange.clone(),
                instrument: position.instrument.clone(),
                strategy: self.id.clone(),
                cid: ClientOrderId::default(),
                side: match position.side {
                    Side::Buy => Side::Sell,
                    Side::Sell => Side::Buy,
                },
                state: RequestOpen {
                    kind: OrderKind::Market,
                    time_in_force: TimeInForce::ImmediateOrCancel,
                    price: Default::default(),
                    quantity: Decimal::from_f64(position.quantity_abs)?,
                },
            })
        });

        (std::iter::empty(), open_requests)
    }
}

impl<State, ExecutionTxs, Risk> OnDisconnectStrategy<State, ExecutionTxs, Risk>
    for DefaultStrategy<State>
{
    type OnDisconnect = ();

    fn on_disconnect(
        _: &mut Engine<State, ExecutionTxs, Self, Risk>,
        _: ExchangeId,
    ) -> Self::OnDisconnect {
    }
}

impl<State, ExecutionTxs, Risk> OnTradingDisabled<State, ExecutionTxs, Risk>
    for DefaultStrategy<State>
{
    type OnTradingDisabled = ();

    fn on_trading_disabled(
        _: &mut Engine<State, ExecutionTxs, Self, Risk>,
    ) -> Self::OnTradingDisabled {
    }
}

#[derive(Debug, Clone)]
pub struct DefaultStrategyState;

impl<ExchangeKey, AssetKey, InstrumentKey>
    Processor<&AccountEvent<ExchangeKey, AssetKey, InstrumentKey>> for DefaultStrategyState
{
    type Output = ();
    fn process(&mut self, _: &AccountEvent<ExchangeKey, AssetKey, InstrumentKey>) -> Self::Output {}
}

impl<InstrumentKey, Kind> Processor<&MarketEvent<InstrumentKey, Kind>> for DefaultStrategyState {
    type Output = ();
    fn process(&mut self, _: &MarketEvent<InstrumentKey, Kind>) -> Self::Output {}
}
