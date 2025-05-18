use crate::{
    Keyed,
    asset::{Asset, AssetIndex, ExchangeAsset},
    exchange::{ExchangeId, ExchangeIndex},
    index::{
        IndexedInstruments, find_asset_by_exchange_and_name_internal, find_exchange_by_exchange_id,
    },
    instrument::{Instrument, InstrumentIndex, spec::OrderQuantityUnits},
};

#[derive(Debug, Default)]
pub struct IndexedInstrumentsBuilder {
    exchanges: Vec<ExchangeId>,
    instruments: Vec<Instrument<ExchangeId, Asset>>,
    assets: Vec<ExchangeAsset<Asset>>,
}

impl IndexedInstrumentsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_instrument(mut self, instrument: Instrument<ExchangeId, Asset>) -> Self {
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
        if let Some(spec) = instrument.spec.as_ref()
            && let OrderQuantityUnits::Asset(asset) = &spec.quantity.unit
        {
            self.assets
                .push(ExchangeAsset::new(instrument.exchange, asset.clone()));
        }

        // Add Instrument
        self.instruments.push(instrument);

        self
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
        Underlying,
        instrument::{
            kind::InstrumentKind,
            name::{InstrumentNameExchange, InstrumentNameInternal},
            quote::InstrumentQuoteAsset,
            spec::{
                InstrumentSpec, InstrumentSpecNotional, InstrumentSpecPrice, InstrumentSpecQuantity,
            },
        },
        test_utils::{exchange_asset, instrument},
    };
    use rust_decimal_macros::dec;

    #[test]
    fn test_builder_basic_spot() {
        // Add single spot instrument
        let indexed = IndexedInstrumentsBuilder::default()
            .add_instrument(instrument(ExchangeId::BinanceSpot, "btc", "usdt"))
            .build();

        // Verify state
        assert_eq!(indexed.exchanges().len(), 1);
        assert_eq!(indexed.assets().len(), 2); // BTC and USDT
        assert_eq!(indexed.instruments().len(), 1);

        // Verify exchanges indexes
        assert_eq!(indexed.exchanges()[0].value, ExchangeId::BinanceSpot);

        // Verify asset indexes
        assert_eq!(
            indexed.assets()[0].value,
            exchange_asset(ExchangeId::BinanceSpot, "btc"),
        );
        assert_eq!(
            indexed.assets()[1].value,
            exchange_asset(ExchangeId::BinanceSpot, "usdt"),
        );

        // Very instrument indexes
        assert_eq!(
            indexed.instruments()[0].value,
            Instrument {
                exchange: Keyed::new(ExchangeIndex(0), ExchangeId::BinanceSpot),
                name_exchange: InstrumentNameExchange::new("btc_usdt"),
                name_internal: InstrumentNameInternal::new("binance_spot-btc_usdt"),
                underlying: Underlying {
                    base: AssetIndex(0),
                    quote: AssetIndex(1),
                },
                quote: InstrumentQuoteAsset::UnderlyingQuote,
                kind: InstrumentKind::Spot,
                spec: None
            }
        );
    }

    #[test]
    fn test_builder_deduplication() {
        // Add same spot instrument twice
        let indexed = IndexedInstrumentsBuilder::default()
            .add_instrument(instrument(ExchangeId::BinanceSpot, "BTC", "USDT"))
            .add_instrument(instrument(ExchangeId::BinanceSpot, "BTC", "USDT"))
            .build();

        // Should deduplicate exchanges and assets, but not instruments
        assert_eq!(indexed.exchanges().len(), 1); // Exchange are de-duped
        assert_eq!(indexed.assets().len(), 2); // BTC and USDT and de-duped
        assert_eq!(indexed.instruments().len(), 1); // Instruments are de-duped
    }

    #[test]
    fn test_builder_multiple_exchanges() {
        // Add instruments from different exchanges
        let indexed = IndexedInstrumentsBuilder::default()
            .add_instrument(instrument(ExchangeId::BinanceSpot, "BTC", "USDT"))
            .add_instrument(instrument(ExchangeId::Coinbase, "BTC", "USD"))
            .build();

        // Should maintain separate indices for same asset on different exchanges
        assert_eq!(indexed.exchanges().len(), 2);
        assert_eq!(indexed.assets().len(), 4); // BTC on both exchanges, USDT and USD
        assert_eq!(indexed.instruments().len(), 2);
    }

    #[test]
    fn test_builder_asset_unit_handling() {
        // Create instrument with asset-based order quantity
        let base_asset = Asset::new_from_exchange("BTC");
        let quote_asset = Asset::new_from_exchange("USDT");

        let instrument = Instrument::new(
            ExchangeId::BinanceSpot,
            "binance_spot_btc_usdt",
            "BTC-USDT",
            Underlying::new(base_asset.clone(), quote_asset.clone()),
            InstrumentQuoteAsset::UnderlyingQuote,
            InstrumentKind::Spot,
            Some(InstrumentSpec {
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
            }),
        );

        let indexed = IndexedInstrumentsBuilder::default()
            .add_instrument(instrument)
            .build();

        // Should index the asset used in OrderQuantityUnits
        assert_eq!(indexed.assets().len(), 2);
        assert_eq!(
            indexed.assets()[0].value,
            exchange_asset(ExchangeId::BinanceSpot, "BTC")
        );
    }

    #[test]
    fn test_builder_ordering() {
        // Add instruments in any order
        let indexed = IndexedInstrumentsBuilder::default()
            .add_instrument(instrument(ExchangeId::BinanceSpot, "BTC", "USDT"))
            .add_instrument(instrument(ExchangeId::Coinbase, "ETH", "USD"))
            .build();

        // Verify exchanges are ordered by input sequence
        assert_eq!(indexed.exchanges()[0].value, ExchangeId::BinanceSpot);
        assert_eq!(indexed.exchanges()[1].value, ExchangeId::Coinbase);

        // Verify exchanges are ordered by input sequence
        assert_eq!(
            indexed.assets()[0].value,
            exchange_asset(ExchangeId::BinanceSpot, "BTC")
        );
        assert_eq!(
            indexed.assets()[1].value,
            exchange_asset(ExchangeId::BinanceSpot, "USDT")
        );
        assert_eq!(
            indexed.assets()[2].value,
            exchange_asset(ExchangeId::Coinbase, "ETH")
        );
        assert_eq!(
            indexed.assets()[3].value,
            exchange_asset(ExchangeId::Coinbase, "USD")
        );

        // Verify instruments are ordered by input sequence
        assert_eq!(
            indexed.instruments()[0].value,
            Instrument {
                exchange: Keyed::new(ExchangeIndex(0), ExchangeId::BinanceSpot),
                name_exchange: InstrumentNameExchange::new("BTC_USDT"),
                name_internal: InstrumentNameInternal::new("binance_spot-btc_usdt"),
                underlying: Underlying {
                    base: AssetIndex(0),
                    quote: AssetIndex(1),
                },
                quote: InstrumentQuoteAsset::UnderlyingQuote,
                kind: InstrumentKind::Spot,
                spec: None
            }
        );

        assert_eq!(
            indexed.instruments()[1].value,
            Instrument {
                exchange: Keyed::new(ExchangeIndex(1), ExchangeId::Coinbase),
                name_exchange: InstrumentNameExchange::new("ETH_USD"),
                name_internal: InstrumentNameInternal::new("coinbase-eth_usd"),
                underlying: Underlying {
                    base: AssetIndex(2),
                    quote: AssetIndex(3),
                },
                quote: InstrumentQuoteAsset::UnderlyingQuote,
                kind: InstrumentKind::Spot,
                spec: None
            }
        );
    }
}
