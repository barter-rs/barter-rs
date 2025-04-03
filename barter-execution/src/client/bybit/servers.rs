use barter_instrument::exchange::ExchangeId;

use super::types::InstrumentCategory;

pub trait BybitServer {
    const EXCHANGE: ExchangeId;
    const CATEGORY: InstrumentCategory;
}

#[derive(Debug, Clone, Copy)]
pub struct SpotServer;

impl BybitServer for SpotServer {
    const EXCHANGE: ExchangeId = ExchangeId::BybitSpot;
    const CATEGORY: InstrumentCategory = InstrumentCategory::Spot;
}

#[derive(Debug, Clone, Copy)]
pub struct FuturesUsdServer;

impl BybitServer for FuturesUsdServer {
    const EXCHANGE: ExchangeId = ExchangeId::BybitPerpetualsUsd;
    const CATEGORY: InstrumentCategory = InstrumentCategory::Linear;
}
