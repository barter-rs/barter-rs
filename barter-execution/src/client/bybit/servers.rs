use barter_instrument::exchange::ExchangeId;

use super::types::InstrumentCategory;

pub trait BybitServer {
    const ID: ExchangeId;
    const CATEGORY: InstrumentCategory;
}

#[derive(Debug, Clone, Copy)]
pub struct SpotServer;

impl BybitServer for SpotServer {
    const ID: ExchangeId = ExchangeId::BybitSpot;
    const CATEGORY: InstrumentCategory = InstrumentCategory::Spot;
}

#[derive(Debug, Clone, Copy)]
pub struct FuturesUsdServer;

impl BybitServer for FuturesUsdServer {
    const ID: ExchangeId = ExchangeId::BybitPerpetualsUsd;
    const CATEGORY: InstrumentCategory = InstrumentCategory::Linear;
}
