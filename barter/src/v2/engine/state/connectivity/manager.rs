use crate::v2::engine::state::connectivity::ConnectivityState;

pub trait ConnectivityManager<ExchangeKey> {
    fn connectivity(&self, key: &ExchangeKey) -> &ConnectivityState;
    fn connectivity_mut(&mut self, key: &ExchangeKey) -> &mut ConnectivityState;
}
