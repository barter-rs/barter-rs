use crate::v2::engine::state::{instrument::InstrumentState, EngineState};
use barter_instrument::{
    instrument::{name::InstrumentNameInternal, InstrumentIndex},
    Underlying,
};
use barter_integration::collection::one_or_many::OneOrMany;
use itertools::Either;
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
    ) -> impl Iterator<
        Item = &'a InstrumentState<Self::Market, Self::ExchangeKey, Self::AssetKey, InstrumentKey>,
    >
    where
        Self::Market: 'a,
        Self::ExchangeKey: 'a,
        Self::AssetKey: 'a,
        InstrumentKey: 'a;

    fn instruments_filtered<'a>(
        &'a self,
        filter: &'a InstrumentFilter<Self::ExchangeKey, Self::AssetKey, InstrumentKey>,
    ) -> impl Iterator<
        Item = &'a InstrumentState<Self::Market, Self::ExchangeKey, Self::AssetKey, InstrumentKey>,
    >
    where
        Self::Market: 'a,
        Self::ExchangeKey: PartialEq + 'a,
        Self::AssetKey: PartialEq + 'a,
        InstrumentKey: PartialEq + 'a,
    {
        use InstrumentFilter::*;
        match filter {
            None => Either::Left(Either::Left(self.instruments())),
            Exchanges(exchanges) => Either::Left(Either::Right(
                self.instruments()
                    .filter(|state| exchanges.contains(&state.instrument.exchange)),
            )),
            Instruments(instruments) => Either::Right(Either::Right(
                self.instruments()
                    .filter(|state| instruments.contains(&state.key)),
            )),
            Underlyings(underlying) => Either::Right(Either::Left(
                self.instruments()
                    .filter(|state| underlying.contains(&state.instrument.underlying)),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub enum InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey> {
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

impl<Market, Strategy, Risk, ExchangeKey, AssetKey> InstrumentStateManager<InstrumentIndex>
    for EngineState<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentIndex>
{
    type ExchangeKey = ExchangeKey;
    type AssetKey = AssetKey;
    type Market = Market;

    fn instrument(
        &self,
        key: &InstrumentIndex,
    ) -> &InstrumentState<Market, ExchangeKey, AssetKey, InstrumentIndex> {
        self.instruments
            .0
            .get_index(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("InstrumentStates does not contain: {key}"))
    }

    fn instrument_mut(
        &mut self,
        key: &InstrumentIndex,
    ) -> &mut InstrumentState<Market, ExchangeKey, AssetKey, InstrumentIndex> {
        self.instruments
            .0
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("InstrumentStates does not contain: {key}"))
    }

    fn instruments<'a>(
        &'a self,
    ) -> impl Iterator<Item = &'a InstrumentState<Market, ExchangeKey, AssetKey, InstrumentIndex>>
    where
        Self::Market: 'a,
        Self::ExchangeKey: 'a,
        Self::AssetKey: 'a,
    {
        self.instruments.0.values()
    }
}

impl<Market, Strategy, Risk, ExchangeKey, AssetKey> InstrumentStateManager<InstrumentNameInternal>
    for EngineState<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentNameInternal>
{
    type ExchangeKey = ExchangeKey;
    type AssetKey = AssetKey;
    type Market = Market;

    fn instrument(
        &self,
        key: &InstrumentNameInternal,
    ) -> &InstrumentState<Market, ExchangeKey, AssetKey, InstrumentNameInternal> {
        self.instruments
            .0
            .get(key)
            .unwrap_or_else(|| panic!("InstrumentStates does not contain: {key}"))
    }

    fn instrument_mut(
        &mut self,
        key: &InstrumentNameInternal,
    ) -> &mut InstrumentState<Market, ExchangeKey, AssetKey, InstrumentNameInternal> {
        self.instruments
            .0
            .get_mut(key)
            .unwrap_or_else(|| panic!("InstrumentStates does not contain: {key}"))
    }

    fn instruments<'a>(
        &'a self,
    ) -> impl Iterator<Item = &'a InstrumentState<Market, ExchangeKey, AssetKey, InstrumentNameInternal>>
    where
        Self::Market: 'a,
        Self::ExchangeKey: 'a,
        Self::AssetKey: 'a,
    {
        self.instruments.0.values()
    }
}
