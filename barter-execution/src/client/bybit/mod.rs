use client::BybitClient;
use servers::{FuturesUsdServer, SpotServer};

mod client;
mod http;
mod servers;
mod types;
pub mod websocket;

pub type BybitSpot = BybitClient<SpotServer>;

// TODO: Test
// pub type BybitFuturesUsd = BybitClient<FuturesUsdServer>;
