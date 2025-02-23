use barter_instrument::{
    Underlying, asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex,
};
use barter_integration::collection::one_or_many::OneOrMany;
use serde::{Deserialize, Serialize};

/// Instrument filter.
///
/// Used to filter instrument-centric data structures such as `InstrumentStates`.
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub enum InstrumentFilter<
    ExchangeKey = ExchangeIndex,
    AssetKey = AssetIndex,
    InstrumentKey = InstrumentIndex,
> {
    None,
    Exchanges(OneOrMany<ExchangeKey>),
    Instruments(OneOrMany<InstrumentKey>),
    Underlyings(OneOrMany<Underlying<AssetKey>>),
}

impl<ExchangeKey, AssetKey, InstrumentKey> InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey> {
    pub fn exchanges(exchanges: impl IntoIterator<Item = ExchangeKey>) -> Self {
        Self::Exchanges(OneOrMany::from_iter(exchanges))
    }

    pub fn instruments(instruments: impl IntoIterator<Item = InstrumentKey>) -> Self {
        Self::Instruments(OneOrMany::from_iter(instruments))
    }

    pub fn underlyings(exchanges: impl IntoIterator<Item = Underlying<AssetKey>>) -> Self {
        Self::Underlyings(OneOrMany::from_iter(exchanges))
    }
}
