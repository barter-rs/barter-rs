use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct InstrumentSpec<AssetKey> {
    pub price: InstrumentSpecPrice,
    pub quantity: InstrumentSpecQuantity<AssetKey>,
    pub notional: InstrumentSpecNotional,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct InstrumentSpecPrice {
    pub min: Decimal,
    pub tick_size: Decimal,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct InstrumentSpecQuantity<AssetKey> {
    pub unit: OrderQuantityUnits<AssetKey>,
    pub min: Decimal,
    pub increment: Decimal,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum OrderQuantityUnits<AssetKey> {
    Asset(AssetKey),
    Contract,
    Quote,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct InstrumentSpecNotional {
    pub min: Decimal,
}
