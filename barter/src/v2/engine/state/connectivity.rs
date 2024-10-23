use crate::v2::execution::error::ConnectivityError;

#[derive(Debug)]
pub struct ConnectivityState {
    market_data: Connection,
    account: Connection,
}

#[derive(Debug)]
pub enum Connection {
    Healthy,
    Unhealthy(ConnectivityError),
    Reconnecting,
}