use crate::v2::instrument::asset::AssetId;
use barter_data::{exchange::ExchangeId, instrument::InstrumentId};
use barter_integration::model::instrument::kind::InstrumentKind;
use serde::{Deserialize, Serialize};

pub mod asset;

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Instrument {
    pub id: InstrumentId,
    pub exchange: ExchangeId,
    pub name_exchange: String,
    pub kind: InstrumentKind,
    pub spec: InstrumentSpec,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct InstrumentSpec {
    pub price: InstrumentSpecPrice,
    pub quantity: InstrumentSpecQuantity,
    pub notional: InstrumentSpecNotional,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct InstrumentSpecPrice {
    pub min: f64,
    pub tick_size: f64,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct InstrumentSpecQuantity {
    pub unit: OrderQuantityUnits,
    pub min: f64,
    pub increment: f64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum OrderQuantityUnits {
    Asset(AssetId),
    Contract,
    Quote,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct InstrumentSpecNotional {
    pub min: f64,
}
