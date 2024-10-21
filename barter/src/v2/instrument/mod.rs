use crate::v2::instrument::asset::{AssetId};
use barter_integration::model::{exchange::ExchangeId};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use chrono::{DateTime, Utc};
use smol_str::SmolStr;
use barter_integration::model::instrument::kind::{FutureContract, OptionContract};

pub mod asset;
pub mod map;

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct KeyedInstrument<Key, Data> {
    pub key: Key,
    pub instrument: Data
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Instrument {
    pub exchange: ExchangeId,
    pub name_internal: SmolStr,
    pub name_exchange: SmolStr,
    pub kind: InstrumentKind,
    pub spec: InstrumentSpec,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub enum InstrumentKind<AssetKey = AssetId> {
    Spot {
        base_asset_id: AssetKey,
    },
    Perpetual {
        base_asset_id: AssetKey,
        quote_asset_id: AssetKey,
        settlement_asset_id: AssetKey,
    },
    Future {
        base_asset_id: AssetKey,
        quote_asset_id: AssetKey,
        settlement_asset_id: AssetKey,
        contract: FutureContract,
    },
    Option {
        base_asset_id: AssetKey,
        quote_asset_id: AssetKey,
        settlement_asset_id: AssetKey,
        contract: OptionContract,
    }
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
