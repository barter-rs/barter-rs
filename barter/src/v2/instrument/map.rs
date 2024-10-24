//
// use fnv::FnvHashMap;
// // Todo:
// //  - Should be able to construct using "global" InstrumentIds, or simply add instruments and
// //    generate an ephemeral / local InstrumentId
// //  - Build this once on startup, then generate market data subscriptions, execution subscriptions
// //  - Move exchange "market"s to barter-integration?
// //    '--> Or new crate "barter-instrument"? or add fetch instrument data in barter-data?
//

use serde::{Deserialize, Serialize};
use barter_instrument::asset::{Asset, AssetIndex, ExchangeAsset};
use barter_instrument::asset::symbol::Symbol;
use barter_instrument::instrument::{InstrumentIndex};
use barter_instrument::instrument::spec::{InstrumentSpec, InstrumentSpecQuantity, OrderQuantityUnits};
use barter_instrument::Keyed;
use crate::v2::execution::map::{ExecutionInstrumentMap};
use crate::v2::instrument::{Instrument, InstrumentKind, Underlying};


// Todo: Make this work with more flexible Instrument / Asset types

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct IndexedInstruments {
    instruments: Vec<Keyed<InstrumentIndex, Instrument<AssetIndex>>>,
    assets: Vec<Keyed<AssetIndex, ExchangeAsset<Asset>>>,
}

impl IndexedInstruments {
    pub fn new<Iter>(instruments: Iter) -> Self 
    where
        Iter: IntoIterator<Item = Instrument<Asset>>
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
     
    pub fn execution_instrument_map(
        &self
    ) -> ExecutionInstrumentMap {
        ExecutionInstrumentMap::new(
            self.assets
                .iter()
                .map(|asset| (
                    asset.key, asset.value.asset.name_exchange.clone()    
                ))
                .collect(),
            self.instruments
                .iter()
                .map(|instrument| (
                    instrument.key, instrument.value.name_exchange.clone()
                ))
                .collect()
        )
    }
}

#[derive(Debug, Default)]
pub struct IndexedInstrumentsBuilder {
    instruments: Vec<Instrument<Asset>>,
    assets: Vec<ExchangeAsset<Asset>>,
}

impl IndexedInstrumentsBuilder {
    pub fn add_instrument(
        &mut self, 
        instrument: Instrument<Asset>
    ) {
        // Add Underlying base
        self.assets.push(ExchangeAsset::new(
            instrument.exchange,
            instrument.underlying.base.clone()
        ));
        
        // Add Underlying quote
        self.assets.push(ExchangeAsset::new(
            instrument.exchange,
            instrument.underlying.quote.clone()
        ));
        
        // If Perpetual, Future, or Option, add settlement asset
        if let Some(settlement_asset) = instrument.kind.settlement_asset() {
            self.assets.push(ExchangeAsset::new(instrument.exchange, settlement_asset.clone()));
        }

        // Add Instrument OrderQuantityUnits if it's defined in asset units 
        // --> likely a duplicate asset, but if so will be filtered during Self::build()
        if let OrderQuantityUnits::Asset(asset) = &instrument.spec.quantity.unit {
            self.assets.push(ExchangeAsset::new(
                instrument.exchange,
                asset.clone()
            ));
        }
        
        // Add Instrument
        self.instruments.push(instrument)
    }
    
    
    pub fn build(mut self) -> IndexedInstruments {
        // Sort & dedup
        self.instruments.sort();
        self.instruments.dedup();
        self.assets.sort();
        self.assets.dedup();
        
        // Index Assets
        let assets = self
            .assets
            .into_iter()
            .enumerate()
            .map(|(index, exchange_asset)| Keyed::new(
                AssetIndex::new(index), 
                exchange_asset
            ))
            .collect::<Vec<_>>();

        // Index Instruments
        let instruments = self
            .instruments
            .into_iter()
            .enumerate()
            .map(|(index, instrument)| {
                let Instrument { 
                    exchange, 
                    name_internal, 
                    name_exchange, 
                    underlying: Underlying { base, quote }, 
                    kind, 
                    spec: InstrumentSpec {
                        price, 
                        quantity: InstrumentSpecQuantity {
                            unit, 
                            min, 
                            increment
                        }, 
                        notional
                    } 
                } = instrument;

                let find_asset_index = |asset: &Asset| assets
                    .iter()
                    .find_map(|indexed| (indexed.value.asset == *asset).then_some(indexed.key))
                    .expect("builder indexed should always contain an Instrument Asset");
                
                let base_index = find_asset_index(&base);
                let quote_index = find_asset_index(&quote);
                let kind = match kind {
                    InstrumentKind::Spot => {
                        InstrumentKind::Spot
                    }
                    InstrumentKind::Perpetual { settlement_asset } => {
                        InstrumentKind::Perpetual {
                            settlement_asset: find_asset_index(&settlement_asset)
                        }
                    }
                    InstrumentKind::Future { settlement_asset, contract } => {
                        InstrumentKind::Future {
                            settlement_asset: find_asset_index(&settlement_asset),
                            contract
                        }
                    }
                    InstrumentKind::Option { settlement_asset, contract } => {
                        InstrumentKind::Option {
                            settlement_asset: find_asset_index(&settlement_asset),
                            contract
                        }
                    }
                };
                let unit = match unit {
                    OrderQuantityUnits::Asset(asset) => OrderQuantityUnits::Asset(find_asset_index(&asset)),
                    OrderQuantityUnits::Contract => OrderQuantityUnits::Contract,
                    OrderQuantityUnits::Quote => OrderQuantityUnits::Quote
                };
                
                Keyed::new(index, Instrument {
                    exchange,
                    name_internal,
                    name_exchange,
                    underlying: Underlying::new(base_index, quote_index),
                    kind,
                    spec: InstrumentSpec {
                        price,
                        quantity: InstrumentSpecQuantity {
                            unit,
                            min,
                            increment,
                        },
                        notional,
                    },
                })
            })
            .collect::<Vec<_>>();
        
        // Todo: need to actually construct IndexMaps now
        
        
        IndexedInstruments {
            instruments
            assets,
        }
    }
}
