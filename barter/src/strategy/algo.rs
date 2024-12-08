use barter_execution::order::{Order, RequestCancel, RequestOpen};

pub trait AlgoStrategy<ExchangeKey, InstrumentKey> {
    type State;
    fn generate_algo_orders(
        &self,
        state: &Self::State,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>>,
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>>,
    );
}
