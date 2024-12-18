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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        instrument::{
            kind::InstrumentKind,
            name::{InstrumentNameExchange, InstrumentNameInternal},
            spec::{
                InstrumentSpec, InstrumentSpecNotional, InstrumentSpecPrice, InstrumentSpecQuantity,
            },
        },
        test_utils::{exchange_asset, instrument, instrument_spec},
        Underlying,
    };
    use rust_decimal_macros::dec;

    #[test]
    fn test_builder_basic_spot() {
        let mut builder = IndexedInstrumentsBuilder::default();

        // Add single spot instrument
        let instrument = instrument(ExchangeId::BinanceSpot, "btc", "usdt");
        builder.add_instrument(instrument);

        let actual = builder.build();

        // Verify state
        assert_eq!(actual.exchanges().len(), 1);
        assert_eq!(actual.assets().len(), 2); // BTC and USDT
        assert_eq!(actual.instruments().len(), 1);

        // Verify exchanges indexes
        assert_eq!(actual.exchanges()[0].value, ExchangeId::BinanceSpot);

        // Verify asset indexes
        assert_eq!(
            actual.assets()[0].value,
            exchange_asset(ExchangeId::BinanceSpot, "btc"),
        );
        assert_eq!(
            actual.assets()[1].value,
            exchange_asset(ExchangeId::BinanceSpot, "usdt"),
        );

        // Very instrument indexes
        assert_eq!(
            actual.instruments()[0].value,
            Instrument {
                exchange: Keyed::new(ExchangeIndex(0), ExchangeId::BinanceSpot),
                name_exchange: InstrumentNameExchange::new("btc_usdt"),
                name_internal: InstrumentNameInternal::new("binance_spot-btc_usdt"),
                underlying: Underlying {
                    base: AssetIndex(0),
                    quote: AssetIndex(1),
                },
                kind: InstrumentKind::Spot,
                spec: instrument_spec()
            }
        );
    }

    #[test]
    fn test_builder_deduplication() {
        let mut builder = IndexedInstrumentsBuilder::default();

        // Add same spot instrument twice
        let instrument1 = instrument(ExchangeId::BinanceSpot, "BTC", "USDT");
        let instrument2 = instrument(ExchangeId::BinanceSpot, "BTC", "USDT");

        builder.add_instrument(instrument1);
        builder.add_instrument(instrument2);

        let indexed = builder.build();

        // Should deduplicate exchanges and assets, but not instruments
        assert_eq!(indexed.exchanges().len(), 1); // Exchange are de-douped
        assert_eq!(indexed.assets().len(), 2); // BTC and USDT and de-douped
        assert_eq!(indexed.instruments().len(), 1); // Instruments are de-douped
    }

    #[test]
    fn test_builder_multiple_exchanges() {
        let mut builder = IndexedInstrumentsBuilder::default();

        // Add instruments from different exchanges
        let instrument1 = instrument(ExchangeId::BinanceSpot, "BTC", "USDT");
        let instrument2 = instrument(ExchangeId::Coinbase, "BTC", "USD");

        builder.add_instrument(instrument1);
        builder.add_instrument(instrument2);

        let indexed = builder.build();

        // Should maintain separate indices for same asset on different exchanges
        assert_eq!(indexed.exchanges().len(), 2);
        assert_eq!(indexed.assets().len(), 4); // BTC on both exchanges, USDT and USD
        assert_eq!(indexed.instruments().len(), 2);
    }

    #[test]
    fn test_builder_asset_unit_handling() {
        let mut builder = IndexedInstrumentsBuilder::default();

        // Create instrument with asset-based order quantity
        let base_asset = Asset::new_from_exchange("BTC");
        let quote_asset = Asset::new_from_exchange("USDT");

        let instrument = Instrument::new(
            ExchangeId::BinanceSpot,
            "binance_spot_btc_usdt",
            "BTC-USDT",
            Underlying::new(base_asset.clone(), quote_asset.clone()),
            InstrumentKind::Spot,
            InstrumentSpec {
                price: InstrumentSpecPrice {
                    min: dec!(0.1),
                    tick_size: dec!(0.1),
                },
                quantity: InstrumentSpecQuantity {
                    unit: OrderQuantityUnits::Asset(base_asset.clone()),
                    min: dec!(0.001),
                    increment: dec!(0.001),
                },
                notional: InstrumentSpecNotional { min: dec!(10) },
            },
        );

        builder.add_instrument(instrument);
        let indexed = builder.build();

        // Should index the asset used in OrderQuantityUnits
        assert_eq!(indexed.assets().len(), 2);
        assert_eq!(
            indexed.assets()[0].value,
            exchange_asset(ExchangeId::BinanceSpot, "BTC")
        );
    }

    #[test]
    fn test_builder_ordering() {
        let mut builder = IndexedInstrumentsBuilder::default();

        // Add instruments in any order
        let instrument1 = instrument(ExchangeId::BinanceSpot, "BTC", "USDT");
        let instrument2 = instrument(ExchangeId::Coinbase, "ETH", "USD");

        builder.add_instrument(instrument1);
        builder.add_instrument(instrument2);

        let actual = builder.build();

        // Verify exchanges are ordered by input sequence
        assert_eq!(actual.exchanges()[0].value, ExchangeId::BinanceSpot);
        assert_eq!(actual.exchanges()[1].value, ExchangeId::Coinbase);

        // Verify exchanges are ordered by input sequence
        assert_eq!(
            actual.assets()[0].value,
            exchange_asset(ExchangeId::BinanceSpot, "BTC")
        );
        assert_eq!(
            actual.assets()[1].value,
            exchange_asset(ExchangeId::BinanceSpot, "USDT")
        );
        assert_eq!(
            actual.assets()[2].value,
            exchange_asset(ExchangeId::Coinbase, "ETH")
        );
        assert_eq!(
            actual.assets()[3].value,
            exchange_asset(ExchangeId::Coinbase, "USD")
        );

        // Verify instruments are ordered by input sequence
        assert_eq!(
            actual.instruments()[0].value,
            Instrument {
                exchange: Keyed::new(ExchangeIndex(0), ExchangeId::BinanceSpot),
                name_exchange: InstrumentNameExchange::new("BTC_USDT"),
                name_internal: InstrumentNameInternal::new("binance_spot-btc_usdt"),
                underlying: Underlying {
                    base: AssetIndex(0),
                    quote: AssetIndex(1),
                },
                kind: InstrumentKind::Spot,
                spec: instrument_spec()
            }
        );

        assert_eq!(
            actual.instruments()[1].value,
            Instrument {
                exchange: Keyed::new(ExchangeIndex(1), ExchangeId::Coinbase),
                name_exchange: InstrumentNameExchange::new("ETH_USD"),
                name_internal: InstrumentNameInternal::new("coinbase-eth_usd"),
                underlying: Underlying {
                    base: AssetIndex(2),
                    quote: AssetIndex(3),
                },
                kind: InstrumentKind::Spot,
                spec: instrument_spec()
            }
        );
    }
}
