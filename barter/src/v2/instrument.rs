use crate::v2::{
    balance::Balance,
    engine::state::{
        asset::{AssetState, AssetStates},
        connectivity::{ConnectivityState, ConnectivityStates},
        instrument::{InstrumentState, InstrumentStates},
        order::Orders,
    },
    error::{BarterError, IndexError},
    execution::map::ExecutionInstrumentMap,
};
use barter_data::subscription::{SubKind, Subscription};
use barter_instrument::{
    asset::{name::AssetNameInternal, Asset, AssetIndex, ExchangeAsset},
    exchange::{ExchangeId, ExchangeIndex},
    instrument::{
        market_data::MarketDataInstrument, name::InstrumentNameInternal, spec::OrderQuantityUnits,
        Instrument, InstrumentIndex,
    },
    Keyed,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct IndexedInstruments {
    pub exchanges: Vec<Keyed<ExchangeIndex, ExchangeId>>,
    pub instruments:
        Vec<Keyed<InstrumentIndex, Instrument<Keyed<ExchangeIndex, ExchangeId>, AssetIndex>>>,
    pub assets: Vec<Keyed<AssetIndex, ExchangeAsset<Asset>>>,
}

impl IndexedInstruments {
    pub fn new<Iter>(instruments: Iter) -> Result<Self, IndexError>
    where
        Iter: IntoIterator<Item = Instrument<ExchangeId, Asset>>,
    {
        instruments
            .into_iter()
            .fold(Self::builder(), |mut builder, instrument| {
                builder.add_instrument(instrument);
                builder
            })
            .build()
    }

    pub fn builder() -> IndexedInstrumentsBuilder {
        IndexedInstrumentsBuilder::default()
    }

    pub fn find_exchange_index(&self, exchange: ExchangeId) -> Result<ExchangeIndex, IndexError> {
        find_exchange_by_exchange_id(&self.exchanges, &exchange)
    }

    pub fn find_asset_index(
        &self,
        exchange: ExchangeId,
        name: &AssetNameInternal,
    ) -> Result<AssetIndex, IndexError> {
        find_asset_by_exchange_and_name_internal(&self.assets, exchange, name)
    }

    pub fn find_instrument_index(
        &self,
        exchange: ExchangeId,
        name: &InstrumentNameInternal,
    ) -> Result<InstrumentIndex, IndexError> {
        self.instruments
            .iter()
            .find_map(|indexed| {
                (indexed.value.exchange.value == exchange && indexed.value.name_internal == *name)
                    .then_some(indexed.key)
            })
            .ok_or(IndexError::AssetIndex(format!(
                "Asset: ({}, {}) must be present in indexed instrument assets: {:?}",
                exchange, name, self.assets
            )))
    }

    pub fn market_data_subscriptions<SubBatchIter, SubIter, Sub>(
        &self,
        batches: SubBatchIter,
    ) -> Result<
        Vec<Vec<Subscription<ExchangeId, Keyed<InstrumentIndex, MarketDataInstrument>>>>,
        BarterError,
    >
    where
        SubBatchIter: IntoIterator<Item = SubIter>,
        SubIter: IntoIterator<Item = Sub>,
        Sub: Into<Subscription<ExchangeId, MarketDataInstrument, SubKind>>,
    {
        batches
            .into_iter()
            .map(|batch| batch
                .into_iter()
                .map(|sub| {
                    let sub = sub.into();

                    let base_index = self.find_asset_index(sub.exchange, &sub.instrument.base)?;
                    let quote_index = self.find_asset_index(sub.exchange, &sub.instrument.quote)?;

                    let find_instrument = |exchange, kind, base, quote| {
                        self.instruments
                            .iter()
                            .find_map(|indexed| {
                                (
                                    indexed.value.exchange.value == exchange
                                        && indexed.value.kind.eq_market_data_instrument_kind(kind)
                                        && indexed.value.underlying.base == base
                                        && indexed.value.underlying.quote == quote
                                ).then_some(indexed.key)
                            })
                            .ok_or(IndexError::InstrumentIndex(format!(
                                "Instrument: ({}, {}, {}, {}) must be present in indexed instruments: {:?}",
                                exchange, kind, base, quote, self.instruments
                            )))
                    };

                    let instrument_index = find_instrument(sub.exchange, &sub.instrument.kind, base_index, quote_index)?;

                    Ok(Subscription {
                        exchange: sub.exchange,
                        instrument: Keyed::new(instrument_index, sub.instrument),
                        kind: sub.kind,
                    })
                })
                .collect()
            )
            .collect()
    }

    pub fn connectivity_states(&self) -> ConnectivityStates {
        ConnectivityStates(
            self.exchanges
                .iter()
                .map(|exchange| (exchange.value, ConnectivityState::default()))
                .collect(),
        )
    }

    pub fn asset_states(&self) -> AssetStates {
        AssetStates(
            self.assets
                .iter()
                .map(|asset| {
                    (
                        ExchangeAsset::new(
                            asset.value.exchange,
                            asset.value.asset.name_internal.clone(),
                        ),
                        AssetState::new(
                            asset.value.asset.clone(),
                            Balance::default(),
                            DateTime::<Utc>::MIN_UTC,
                        ),
                    )
                })
                .collect(),
        )
    }

    pub fn instrument_states<Market>(
        &self,
    ) -> InstrumentStates<Market, ExchangeIndex, AssetIndex, InstrumentIndex>
    where
        Market: Default,
    {
        InstrumentStates(
            self.instruments
                .iter()
                .map(|instrument| {
                    let exchange_index = instrument.value.exchange.key;
                    (
                        instrument.value.name_internal.clone(),
                        InstrumentState::new(
                            instrument.key,
                            instrument.value.clone().map_exchange_key(exchange_index),
                            None,
                            Orders::default(),
                            Market::default(),
                        ),
                    )
                })
                .collect(),
        )
    }

    pub fn execution_instrument_map(
        &self,
        exchange: ExchangeId,
    ) -> Result<ExecutionInstrumentMap, IndexError> {
        let exchange_index = self
            .exchanges
            .iter()
            .find_map(|keyed_exchange| {
                (keyed_exchange.value == exchange).then_some(keyed_exchange.key)
            })
            .ok_or_else(|| {
                IndexError::ExchangeIndex(format!(
                    "IndexedInstrument does not contain index for: {exchange}"
                ))
            })?;

        Ok(ExecutionInstrumentMap::new(
            Keyed::new(exchange_index, exchange),
            self.assets
                .iter()
                .filter_map(|asset| {
                    (asset.value.exchange == exchange)
                        .then_some((asset.key, asset.value.asset.name_exchange.clone()))
                })
                .collect(),
            self.instruments
                .iter()
                .filter_map(|instrument| {
                    (instrument.value.exchange.value == exchange)
                        .then_some((instrument.key, instrument.value.name_exchange.clone()))
                })
                .collect(),
        ))
    }
}

#[derive(Debug, Default)]
pub struct IndexedInstrumentsBuilder {
    exchanges: Vec<ExchangeId>,
    instruments: Vec<Instrument<ExchangeId, Asset>>,
    assets: Vec<ExchangeAsset<Asset>>,
}

impl IndexedInstrumentsBuilder {
    pub fn add_instrument(&mut self, instrument: Instrument<ExchangeId, Asset>) {
        // Add ExchangeId
        self.exchanges.push(instrument.exchange);

        // Add Underlying base
        self.assets.push(ExchangeAsset::new(
            instrument.exchange,
            instrument.underlying.base.clone(),
        ));

        // Add Underlying quote
        self.assets.push(ExchangeAsset::new(
            instrument.exchange,
            instrument.underlying.quote.clone(),
        ));

        // If Perpetual, Future, or Option, add settlement asset
        if let Some(settlement_asset) = instrument.kind.settlement_asset() {
            self.assets.push(ExchangeAsset::new(
                instrument.exchange,
                settlement_asset.clone(),
            ));
        }

        // Add Instrument OrderQuantityUnits if it's defined in asset units
        // --> likely a duplicate asset, but if so will be filtered during Self::build()
        if let OrderQuantityUnits::Asset(asset) = &instrument.spec.quantity.unit {
            self.assets
                .push(ExchangeAsset::new(instrument.exchange, asset.clone()));
        }

        // Add Instrument
        self.instruments.push(instrument)
    }

    pub fn build(mut self) -> Result<IndexedInstruments, IndexError> {
        // Sort & dedup
        self.exchanges.sort();
        self.exchanges.dedup();
        self.instruments.sort();
        self.instruments.dedup();
        self.assets.sort();
        self.assets.dedup();

        // Index Exchanges
        let exchanges = self
            .exchanges
            .into_iter()
            .enumerate()
            .map(|(index, exchange)| Keyed::new(ExchangeIndex::new(index), exchange))
            .collect::<Vec<_>>();

        // Index Assets
        let assets = self
            .assets
            .into_iter()
            .enumerate()
            .map(|(index, exchange_asset)| Keyed::new(AssetIndex::new(index), exchange_asset))
            .collect::<Vec<_>>();

        // Index Instruments (also maps any Instrument AssetKeys -> AssetIndex)
        let instruments = self
            .instruments
            .into_iter()
            .enumerate()
            .map(|(index, instrument)| {
                let exchange_id = instrument.exchange;
                let exchange_key = find_exchange_by_exchange_id(&exchanges, &exchange_id)?;

                let instrument = instrument.map_exchange_key(Keyed::new(exchange_key, exchange_id));

                let instrument = instrument.map_asset_key_with_lookup(|asset: &Asset| {
                    find_asset_by_exchange_and_name_internal(
                        &assets,
                        exchange_id,
                        &asset.name_internal,
                    )
                })?;

                Ok(Keyed::new(InstrumentIndex::new(index), instrument))
            })
            .collect::<Result<Vec<_>, IndexError>>()?;

        Ok(IndexedInstruments {
            exchanges,
            instruments,
            assets,
        })
    }
}

fn find_exchange_by_exchange_id(
    haystack: &[Keyed<ExchangeIndex, ExchangeId>],
    needle: &ExchangeId,
) -> Result<ExchangeIndex, IndexError> {
    haystack
        .iter()
        .find_map(|indexed| (indexed.value == *needle).then_some(indexed.key))
        .ok_or(IndexError::ExchangeIndex(format!(
            "Exchange: {} must be present in indexed instrument exchanges: {:?}",
            needle, haystack
        )))
}

fn find_asset_by_exchange_and_name_internal(
    haystack: &[Keyed<AssetIndex, ExchangeAsset<Asset>>],
    needle_exchange: ExchangeId,
    needle_name: &AssetNameInternal,
) -> Result<AssetIndex, IndexError> {
    haystack
        .iter()
        .find_map(|indexed| {
            (indexed.value.exchange == needle_exchange
                && indexed.value.asset.name_internal == *needle_name)
                .then_some(indexed.key)
        })
        .ok_or(IndexError::AssetIndex(format!(
            "Asset: ({}, {}) must be present in indexed instrument assets: {:?}",
            needle_exchange, needle_name, haystack
        )))
}
