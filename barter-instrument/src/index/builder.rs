use crate::{
    asset::{Asset, AssetIndex, ExchangeAsset},
    exchange::{ExchangeId, ExchangeIndex},
    index::{
        find_asset_by_exchange_and_name_internal, find_exchange_by_exchange_id, IndexedInstruments,
    },
    instrument::{spec::OrderQuantityUnits, Instrument, InstrumentIndex},
    Keyed,
};

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

    pub fn build(mut self) -> IndexedInstruments {
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
                let exchange_key = find_exchange_by_exchange_id(&exchanges, &exchange_id)
                    .expect("every exchange related to every instrument has been added");

                let instrument = instrument.map_exchange_key(Keyed::new(exchange_key, exchange_id));

                let instrument = instrument
                    .map_asset_key_with_lookup(|asset: &Asset| {
                        find_asset_by_exchange_and_name_internal(
                            &assets,
                            exchange_id,
                            &asset.name_internal,
                        )
                    })
                    .expect("every asset related to every instrument has been added");

                Keyed::new(InstrumentIndex::new(index), instrument)
            })
            .collect();

        IndexedInstruments {
            exchanges,
            instruments,
            assets,
        }
    }
}
