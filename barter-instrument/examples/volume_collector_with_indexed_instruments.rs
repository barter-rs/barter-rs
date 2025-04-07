use barter_instrument::{
    Keyed, Underlying,
    asset::{Asset, AssetIndex, ExchangeAsset, name::AssetNameInternal},
    exchange::{ExchangeId, ExchangeIndex},
    index::IndexedInstruments,
    instrument::{Instrument, InstrumentIndex, name::InstrumentNameInternal},
};

use rust_decimal::{Decimal, prelude::Zero};

type VolumeSum = Decimal;
type FnvIndexMap<K, V> = indexmap::IndexMap<K, V, fnv::FnvBuildHasher>;

#[derive(Debug, Clone, PartialEq)]
struct VolumeCollector<K, V> {
    key: K,
    value: V,
    sum_of_volume: VolumeSum,
}

impl<K, V> VolumeCollector<K, V> {
    pub fn new(key: K, value: V) -> Self {
        Self {
            key,
            value,
            sum_of_volume: Decimal::zero(),
        }
    }
}

type InstrumentVolumeCollector =
    VolumeCollector<InstrumentIndex, Instrument<Keyed<ExchangeIndex, ExchangeId>, AssetIndex>>;
type ExchangeVolumeCollector = VolumeCollector<ExchangeIndex, ExchangeId>;
type AssetVolumeCollector = VolumeCollector<AssetIndex, ExchangeAsset<Asset>>;

// `indexed_instruments` - Reference to IndexedInstruments containing what instruments,
// exchanges and assets are being tracked.
fn generate_instrument_volume_collectors(
    indexed_instruments: &IndexedInstruments,
) -> FnvIndexMap<InstrumentNameInternal, InstrumentVolumeCollector> {
    indexed_instruments
        .instruments()
        .iter()
        .map(|instrument| {
            (
                instrument.value.name_internal.clone(),
                InstrumentVolumeCollector::new(instrument.key, instrument.value.clone()),
            )
        })
        .collect()
}

fn generate_asset_volume_collectors(
    indexed_instruments: &IndexedInstruments,
) -> FnvIndexMap<ExchangeAsset<AssetNameInternal>, AssetVolumeCollector> {
    indexed_instruments
        .assets()
        .iter()
        .map(|asset| {
            (
                ExchangeAsset::new(
                    asset.value.exchange,
                    asset.value.asset.name_internal.clone(),
                ),
                AssetVolumeCollector::new(asset.key, asset.value.clone()),
            )
        })
        .collect()
}

fn generate_exchange_volume_collectors(
    indexed_instruments: &IndexedInstruments,
) -> FnvIndexMap<ExchangeId, ExchangeVolumeCollector> {
    indexed_instruments
        .exchanges()
        .iter()
        .map(|exchange| {
            (
                exchange.value,
                ExchangeVolumeCollector::new(exchange.key, exchange.value),
            )
        })
        .collect()
}

#[derive(Default)]

struct TradeVolumeCollector {
    instrument_volumes: FnvIndexMap<InstrumentNameInternal, InstrumentVolumeCollector>,
    asset_volumes: FnvIndexMap<ExchangeAsset<AssetNameInternal>, AssetVolumeCollector>,
    exchange_volumes: FnvIndexMap<ExchangeId, ExchangeVolumeCollector>,
}

impl TradeVolumeCollector {
    pub fn new(indexed_instruments: &IndexedInstruments) -> Self {
        Self {
            instrument_volumes: generate_instrument_volume_collectors(indexed_instruments),
            asset_volumes: generate_asset_volume_collectors(indexed_instruments),
            exchange_volumes: generate_exchange_volume_collectors(indexed_instruments),
        }
    }

    pub fn collect_trade(
        &mut self,
        instrument_index: &InstrumentIndex,
        exchange_index: &ExchangeIndex,
        base_asset_index: &AssetIndex,
        quote_asset_index: &AssetIndex,
        volume: f64,
    ) {
        // Update instrument volume collector using instrument_index O(1)
        let instrument_collector = self
            .instrument_volumes
            .get_index_mut(instrument_index.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| {
                panic!("InstrumentVolumes does not contain: {:?}", instrument_index)
            });
        instrument_collector.sum_of_volume += Decimal::try_from(volume).unwrap();

        // Update exchange volume collector using exchange_index O(1)
        let exchange_collector = self
            .exchange_volumes
            .get_index_mut(exchange_index.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("ExchangeVolumes does not contain: {:?}", exchange_index));
        exchange_collector.sum_of_volume += Decimal::try_from(volume).unwrap();

        // Update asset volume collector using asset_index O(1)
        let asset_collector = self
            .asset_volumes
            .get_index_mut(base_asset_index.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("AssetVolumes does not contain: {:?}", base_asset_index));
        asset_collector.sum_of_volume += Decimal::try_from(volume).unwrap();

        // Update asset volume collector using asset_index O(1)
        let asset_collector = self
            .asset_volumes
            .get_index_mut(quote_asset_index.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("AssetVolumes does not contain: {:?}", quote_asset_index));
        asset_collector.sum_of_volume += Decimal::try_from(volume).unwrap();
    }

    pub fn trade_summary(&self) {
        println!("--- Trade Volume Summary ---");

        println!("\n📌 Per Instrument:");
        for (name, collector) in &self.instrument_volumes {
            println!(
                "Instrument: {}, Total Volume in USD: {}",
                name, collector.sum_of_volume
            );
        }

        println!("\n📌 Per Asset:");
        for (asset, collector) in &self.asset_volumes {
            println!(
                "Asset: {} on Exchange: {}, Total Volume in USD: {}",
                asset.asset, asset.exchange, collector.sum_of_volume
            );
        }

        println!("\n📌 Per Exchange:");
        for (exchange, collector) in &self.exchange_volumes {
            println!(
                "Exchange: {}, Total Volume in USD: {}",
                exchange, collector.sum_of_volume
            );
        }

        println!("----------------------------");
    }
}

fn main() {
    // Creating Indexed Instruments with the Indexed Instruments Builder
    let indexed_instruments = IndexedInstruments::builder()
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            None,
        ))
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_eth_usdt",
            "ETHUSDT",
            Underlying::new("eth", "usdt"),
            None,
        ))
        .add_instrument(Instrument::spot(
            ExchangeId::Coinbase,
            "coinbase_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            None,
        ))
        .build();

    // The Trade Volume Collector is built using IndexedInstruments, which contains
    // all the instruments, exchanges, and assets used throughout the system's lifetime.
    // Each property (instrument, exchange, or asset) has a unique corresponding ID
    // that matches its position in a vector of the respective elements.
    //
    // During construction, the Trade Volume Collector consumes the IndexedInstruments
    // and creates individual collectors for each exchange, instrument, and asset.
    // These collectors are then stored in an IndexedMap, where each property's ID
    // directly corresponds to its position in the respective IndexedMap.

    let mut traded_volume_collector = TradeVolumeCollector::new(&indexed_instruments);

    // Extracting the IDs of each property from the IndexedInstruments collection.
    // These IDs enable O(1) lookups within the system, allowing components
    // to efficiently retrieve the corresponding properties when initialized
    // with IndexedInstruments.
    let binance_spot_btc_usdt_index = indexed_instruments
        .find_instrument_index(
            ExchangeId::BinanceSpot,
            &InstrumentNameInternal::from("binance_spot_btc_usdt"),
        )
        .unwrap();
    let binance_spot_eth_usdt_index = indexed_instruments
        .find_instrument_index(
            ExchangeId::BinanceSpot,
            &InstrumentNameInternal::from("binance_spot_eth_usdt"),
        )
        .unwrap();
    let coinbase_spot_btc_usdt_index = indexed_instruments
        .find_instrument_index(
            ExchangeId::Coinbase,
            &InstrumentNameInternal::from("coinbase_btc_usdt"),
        )
        .unwrap();

    let binance_spot_exchange_index = indexed_instruments
        .find_exchange_index(ExchangeId::BinanceSpot)
        .unwrap();

    let coinbase_exchange_index = indexed_instruments
        .find_exchange_index(ExchangeId::Coinbase)
        .unwrap();

    let btc_asset_binance_spot_index = indexed_instruments
        .find_asset_index(ExchangeId::BinanceSpot, &AssetNameInternal::from("BTC"))
        .unwrap();
    let usdt_asset_binance_spot_index = indexed_instruments
        .find_asset_index(ExchangeId::BinanceSpot, &AssetNameInternal::from("USDT"))
        .unwrap();
    let eth_asset_binance_spot_index = indexed_instruments
        .find_asset_index(ExchangeId::BinanceSpot, &AssetNameInternal::from("ETH"))
        .unwrap();
    let btc_asset_coinbase_index = indexed_instruments
        .find_asset_index(ExchangeId::Coinbase, &AssetNameInternal::from("BTC"))
        .unwrap();
    let usdt_asset_coinbase_index = indexed_instruments
        .find_asset_index(ExchangeId::Coinbase, &AssetNameInternal::from("USDT"))
        .unwrap();

    // Using the extracted IDs to verify their consistency with those in the Trade Volume Collector.
    // Generating a few trades to update the Trade Volume Collector's state, ensuring that all updates
    // remain O(1).
    //
    // This is the most critical aspect: throughout the system's lifetime, the indexes must remain stable
    // to leverage IndexedInstruments and O(1) lookups.
    //
    // If new instruments are dynamically allocated, IndexedInstruments can no longer be used,
    // and traditional hash maps must be employed instead.

    traded_volume_collector.collect_trade(
        &binance_spot_btc_usdt_index,
        &binance_spot_exchange_index,
        &btc_asset_binance_spot_index,
        &usdt_asset_binance_spot_index,
        50.0,
    );
    traded_volume_collector.collect_trade(
        &binance_spot_btc_usdt_index,
        &binance_spot_exchange_index,
        &btc_asset_binance_spot_index,
        &usdt_asset_binance_spot_index,
        30.0,
    );
    traded_volume_collector.collect_trade(
        &binance_spot_eth_usdt_index,
        &binance_spot_exchange_index,
        &eth_asset_binance_spot_index,
        &usdt_asset_binance_spot_index,
        70.0,
    );
    traded_volume_collector.collect_trade(
        &coinbase_spot_btc_usdt_index,
        &coinbase_exchange_index,
        &btc_asset_coinbase_index,
        &usdt_asset_coinbase_index,
        20.0,
    );
    traded_volume_collector.collect_trade(
        &coinbase_spot_btc_usdt_index,
        &coinbase_exchange_index,
        &btc_asset_coinbase_index,
        &usdt_asset_coinbase_index,
        65.0,
    );

    //Displaying the trading summary
    traded_volume_collector.trade_summary();
}
