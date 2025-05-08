use derive_more::Constructor;
use rust_decimal::Decimal;
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Constructor,
)]
pub struct InstrumentSpec<AssetKey> {
    pub price: InstrumentSpecPrice,
    pub quantity: InstrumentSpecQuantity<AssetKey>,
    pub notional: InstrumentSpecNotional,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Constructor,
)]
pub struct InstrumentSpecPrice {
    pub min: Decimal,
    pub tick_size: Decimal,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Constructor,
)]
pub struct InstrumentSpecQuantity<AssetKey> {
    pub unit: OrderQuantityUnits<AssetKey>,
    pub min: Decimal,
    pub increment: Decimal,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum OrderQuantityUnits<AssetKey> {
    Asset(AssetKey),
    Contract,
    Quote,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Constructor
)]
pub struct InstrumentSpecNotional {
    pub min: Decimal,
}
