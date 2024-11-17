use crate::v2::engine::state::instrument::InstrumentState;
use barter_instrument::Underlying;
use barter_integration::collection::one_or_many::OneOrMany;
use serde::{Deserialize, Serialize};

pub trait InstrumentStateManager<InstrumentKey> {
    type ExchangeKey;
    type AssetKey;
    type Market;

    fn instrument(
        &self,
        key: &InstrumentKey,
    ) -> &InstrumentState<Self::Market, Self::ExchangeKey, Self::AssetKey, InstrumentKey>;

    fn instrument_mut(
        &mut self,
        key: &InstrumentKey,
    ) -> &mut InstrumentState<Self::Market, Self::ExchangeKey, Self::AssetKey, InstrumentKey>;

    fn instruments<'a>(
        &'a self,
        _filter: &'a InstrumentFilter<Self::ExchangeKey, Self::AssetKey, InstrumentKey>,
    ) -> impl Iterator<
        Item = &'a InstrumentState<Self::Market, Self::ExchangeKey, Self::AssetKey, InstrumentKey>,
    > {
        std::iter::empty()
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub enum InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey> {
    None,
    Exchanges(OneOrMany<ExchangeKey>),
    Instruments(OneOrMany<InstrumentKey>),
    Underlying(OneOrMany<Underlying<AssetKey>>),
}
