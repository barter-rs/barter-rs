use crate::v2::{
    engine::{
        state::{
            asset::AssetStates,
            instrument::{manager::InstrumentFilter, InstrumentStates},
        },
        Engine, Processor,
    },
    execution::AccountEvent,
    order::{ClientOrderId, Order, OrderKind, RequestCancel, RequestOpen, TimeInForce},
    strategy::{
        algo::AlgoStrategy, close_positions::ClosePositionsStrategy,
        on_disconnect::OnDisconnectStrategy,
    },
};
use barter_data::event::MarketEvent;
use barter_instrument::{exchange::ExchangeId, Side};

pub mod algo;
pub mod close_positions;
pub mod on_disconnect;

// Todo: RequestOpen should probably be an enum, since Price is not relevant for OrderKind::Market

pub trait Strategy {
    type State;
}

#[derive(Debug, Clone)]
pub struct DefaultStrategy;

impl Strategy for DefaultStrategy {
    type State = DefaultStrategyState;
}

impl<MarketState, ExchangeKey, AssetKey, InstrumentKey>
    AlgoStrategy<MarketState, ExchangeKey, AssetKey, InstrumentKey> for DefaultStrategy
{
    fn generate_algo_orders(
        &self,
        _: &Self::State,
        _: &AssetStates,
        _: &InstrumentStates<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>>,
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>>,
    ) {
        (std::iter::empty(), std::iter::empty())
    }
}

impl<MarketState, ExchangeKey, AssetKey, InstrumentKey>
    ClosePositionsStrategy<MarketState, ExchangeKey, AssetKey, InstrumentKey> for DefaultStrategy
where
    ExchangeKey: PartialEq + Clone,
    InstrumentKey: PartialEq + Clone,
{
    fn close_positions_requests<'a>(
        &'a self,
        _: &'a Self::State,
        _: &'a AssetStates,
        instrument_states: &'a InstrumentStates<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
        filter: &'a InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>> + 'a,
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>> + 'a,
    )
    where
        ExchangeKey: 'a,
        InstrumentKey: 'a,
    {
        let open_requests = instrument_states.states(filter).filter_map(|state| {
            if state.position.quantity_net.is_zero() {
                return None;
            }

            Some(Order {
                exchange: state.instrument.exchange.clone(),
                instrument: state.position.instrument.clone(),
                cid: ClientOrderId::default(),
                side: if state.position.quantity_net.is_sign_positive() {
                    Side::Sell
                } else {
                    Side::Buy
                },
                state: RequestOpen {
                    kind: OrderKind::Market,
                    time_in_force: TimeInForce::ImmediateOrCancel,
                    price: Default::default(),
                    quantity: state.position.quantity_net.abs(),
                },
            })
        });

        (std::iter::empty(), open_requests)
    }
}

impl<State, ExecutionTxs, Risk> OnDisconnectStrategy<State, ExecutionTxs, Risk>
    for DefaultStrategy
{
    type Output = ();

    fn on_disconnect(
        engine: &mut Engine<State, ExecutionTxs, Self, Risk>,
        exchange: ExchangeId,
    ) -> Self::Output {
        todo!()
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
