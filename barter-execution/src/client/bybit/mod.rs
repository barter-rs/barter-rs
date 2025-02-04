use client::BybitClient;
use servers::{FuturesUsdServer, SpotServer};

use crate::ApiCredentials;

mod client;
mod http;
mod servers;
mod types;
mod websocket;

#[derive(Debug, Clone)]
pub struct BybitConfig {
    credentials: ApiCredentials,
}

pub type BybitSpot = BybitClient<SpotServer>;

pub type BybitFuturesUsd = BybitClient<FuturesUsdServer>;
