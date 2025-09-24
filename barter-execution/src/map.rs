use crate::error::KeyError;
use barter_instrument::{
    Keyed,
    asset::{Asset, AssetIndex, ExchangeAsset, name::AssetNameExchange},
    exchange::{ExchangeId, ExchangeIndex},
    index::{IndexedInstruments, error::IndexError},
    instrument::{Instrument, InstrumentIndex, name::InstrumentNameExchange},
};
use barter_integration::collection::FnvIndexSet;
use fnv::FnvHashMap;

/// Indexed instrument map used to associate the internal Barter representation of instruments and
/// assets with the [`ExecutionClient`](super::client::ExecutionClient) representation.
///
/// Similarly, when the execution manager received an [`AccountEvent`](super::AccountEvent)
/// from the execution API, it needs to determine the internal representation of the associated
/// assets and instruments.
///
/// eg/ `InstrumentNameExchange("XBT-USDT")` <--> `InstrumentIndex(1)` <br>
/// eg/ `AssetNameExchange("XBT")` <--> `AssetIndex(1)`
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExecutionInstrumentMap {
    /// The exchange associated with this execution map.
    pub exchange: Keyed<ExchangeIndex, ExchangeId>,
    /// Collection of assets available by the engine with their
    /// exchange-specific representations. This holds all indexed assets.
    pub assets: FnvIndexSet<ExchangeAsset<Asset>>,
    /// Collection of instruments available by the engine. This holds all
    /// indexed instruments.
    pub instruments: FnvIndexSet<Instrument<Keyed<ExchangeIndex, ExchangeId>, AssetIndex>>,
    /// Map from exchange-specific asset names to internal asset indices for
    /// fast lookups.
    pub asset_names: FnvHashMap<AssetNameExchange, AssetIndex>,
    /// Map from exchange-specific instrument names to internal instrument
    /// indices for fast lookups.
    pub instrument_names: FnvHashMap<InstrumentNameExchange, InstrumentIndex>,
}

impl ExecutionInstrumentMap {
    /// Construct a new [`Self`] using the provided indexed assets and instruments.
    pub fn new(
        exchange: Keyed<ExchangeIndex, ExchangeId>,
        instruments: &IndexedInstruments,
    ) -> Self {
        let asset_names = instruments
            .assets()
            .iter()
            .filter_map(|Keyed { key, value }| {
                (value.exchange == exchange.value)
                    .then_some((value.asset.name_exchange.clone(), *key))
            })
            .collect();

        let assets = instruments
            .assets()
            .iter()
            .map(|Keyed { value, .. }| value.clone())
            .collect();

        let instrument_names = instruments
            .instruments()
            .iter()
            .filter_map(|Keyed { key, value }| {
                (value.exchange.value == exchange.value)
                    .then_some((value.name_exchange.clone(), *key))
            })
            .collect();

        let instruments = instruments
            .instruments()
            .iter()
            .map(|Keyed { value, .. }| value.clone())
            .collect();

        Self {
            exchange,
            asset_names,
            instrument_names,
            assets,
            instruments,
        }
    }

    pub fn exchange_assets(&self) -> impl Iterator<Item = &AssetNameExchange> {
        self.asset_names.iter().map(|(asset, _)| asset)
    }

    pub fn exchange_instruments(&self) -> impl Iterator<Item = &InstrumentNameExchange> {
        self.instrument_names
            .iter()
            .map(|(instrument, _)| instrument)
    }

    pub fn find_exchange_id(&self, exchange: ExchangeIndex) -> Result<ExchangeId, KeyError> {
        if self.exchange.key == exchange {
            Ok(self.exchange.value)
        } else {
            Err(KeyError::ExchangeId(format!(
                "ExecutionInstrumentMap does not contain {exchange}"
            )))
        }
    }

    pub fn find_exchange_index(&self, exchange: ExchangeId) -> Result<ExchangeIndex, IndexError> {
        if self.exchange.value == exchange {
            Ok(self.exchange.key)
        } else {
            Err(IndexError::ExchangeIndex(format!(
                "ExecutionInstrumentMap does not contain {exchange}"
            )))
        }
    }

    pub fn find_asset_name_exchange(
        &self,
        asset: AssetIndex,
    ) -> Result<&AssetNameExchange, KeyError> {
        self.assets
            .get_index(asset.index())
            .ok_or_else(|| {
                KeyError::AssetKey(format!("ExecutionInstrumentMap does not contain: {asset}"))
            })
            .map(|asset| &asset.asset.name_exchange)
    }

    pub fn find_asset_index(&self, asset: &AssetNameExchange) -> Result<AssetIndex, IndexError> {
        self.asset_names.get(asset).copied().ok_or_else(|| {
            IndexError::AssetIndex(format!("ExecutionInstrumentMap does not contain: {asset}"))
        })
    }

    pub fn find_instrument_name_exchange(
        &self,
        instrument: InstrumentIndex,
    ) -> Result<&InstrumentNameExchange, KeyError> {
        self.instruments
            .get_index(instrument.index())
            .ok_or_else(|| {
                KeyError::InstrumentKey(format!(
                    "ExecutionInstrumentMap does not contain: {instrument}"
                ))
            })
            .map(|instrument| &instrument.name_exchange)
    }

    pub fn find_instrument_index(
        &self,
        instrument: &InstrumentNameExchange,
    ) -> Result<InstrumentIndex, IndexError> {
        self.instrument_names
            .get(instrument)
            .copied()
            .ok_or_else(|| {
                IndexError::InstrumentIndex(format!(
                    "ExecutionInstrumentMap does not contain: {instrument}"
                ))
            })
    }
}

pub fn generate_execution_instrument_map(
    instruments: &IndexedInstruments,
    exchange: ExchangeId,
) -> Result<ExecutionInstrumentMap, IndexError> {
    let exchange_index = instruments
        .exchanges()
        .iter()
        .find_map(|keyed_exchange| (keyed_exchange.value == exchange).then_some(keyed_exchange.key))
        .ok_or_else(|| {
            IndexError::ExchangeIndex(format!(
                "IndexedInstrument does not contain index for: {exchange}"
            ))
        })?;

    Ok(ExecutionInstrumentMap::new(
        Keyed::new(exchange_index, exchange),
        instruments,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use barter_instrument::{exchange::ExchangeId, test_utils};

    fn indexed_instruments() -> IndexedInstruments {
        let instruments = vec![
            test_utils::instrument(ExchangeId::BinanceSpot, "BTC", "ETH"),
            test_utils::instrument(ExchangeId::Coinbase, "BTC", "ETH"),
            test_utils::instrument(ExchangeId::Kraken, "USDC", "USDT"),
        ];

        IndexedInstruments::new(instruments)
    }

    #[test]
    fn test_find_exchange_id() {
        let instruments = indexed_instruments();
        let kraken = generate_execution_instrument_map(&instruments, ExchangeId::Kraken).unwrap();

        let exchange_id = kraken.find_exchange_id(kraken.exchange.key).unwrap();
        assert_eq!(exchange_id, ExchangeId::Kraken);
    }

    #[test]
    fn test_find_exchange_index() {
        let instruments = indexed_instruments();
        let kraken = generate_execution_instrument_map(&instruments, ExchangeId::Kraken).unwrap();

        let exchange_index = kraken.find_exchange_index(ExchangeId::Kraken).unwrap();
        assert_eq!(exchange_index, kraken.exchange.key);
    }

    #[test]
    fn test_find_exchange_id_error() {
        let instruments = indexed_instruments();
        let kraken = generate_execution_instrument_map(&instruments, ExchangeId::Kraken).unwrap();

        // Create a different exchange index that doesn't match
        let binance_index = instruments
            .exchanges()
            .iter()
            .find(|ex| ex.value == ExchangeId::BinanceSpot)
            .map(|ex| ex.key)
            .unwrap();

        let result = kraken.find_exchange_id(binance_index);
        assert!(result.is_err());
        assert!(matches!(result, Err(KeyError::ExchangeId(_))));
    }

    #[test]
    fn test_find_exchange_index_error() {
        let instruments = indexed_instruments();
        let kraken = generate_execution_instrument_map(&instruments, ExchangeId::Kraken).unwrap();

        let result = kraken.find_exchange_index(ExchangeId::BinanceSpot);
        assert!(result.is_err());
        assert!(matches!(result, Err(IndexError::ExchangeIndex(_))));
    }

    #[test]
    fn test_find_asset_name_exchange() {
        let instruments = indexed_instruments();
        let kraken = generate_execution_instrument_map(&instruments, ExchangeId::Kraken).unwrap();

        let usdt = test_utils::asset("USDT");
        let usdt_index = instruments
            .find_asset_index(ExchangeId::Kraken, &usdt.name_internal)
            .unwrap();

        let usdt_exchange_name = kraken.find_asset_name_exchange(usdt_index).unwrap();
        assert_eq!(usdt_exchange_name, &usdt.name_exchange);
    }

    #[test]
    fn test_find_asset_index() {
        let instruments = indexed_instruments();
        let kraken = generate_execution_instrument_map(&instruments, ExchangeId::Kraken).unwrap();

        let usdc = test_utils::asset("USDC");
        let asset_index = kraken.find_asset_index(&usdc.name_exchange).unwrap();

        let expected_index = instruments
            .find_asset_index(ExchangeId::Kraken, &usdc.name_internal)
            .unwrap();
        assert_eq!(asset_index, expected_index);
    }

    #[test]
    fn test_find_asset_index_error() {
        let instruments = indexed_instruments();
        let kraken = generate_execution_instrument_map(&instruments, ExchangeId::Kraken).unwrap();

        let btc = test_utils::asset("BTC");
        let result = kraken.find_asset_index(&btc.name_exchange);
        assert!(result.is_err());
        assert!(matches!(result, Err(IndexError::AssetIndex(_))));
    }

    #[test]
    fn test_find_asset_name_exchange_error() {
        let instruments = indexed_instruments();
        let kraken = generate_execution_instrument_map(&instruments, ExchangeId::Kraken).unwrap();

        // Try to find asset with invalid index
        let invalid_index = AssetIndex::new(999);
        let result = kraken.find_asset_name_exchange(invalid_index);
        assert!(result.is_err());
        assert!(matches!(result, Err(KeyError::AssetKey(_))));
    }

    #[test]
    fn test_find_instrument_name_exchange() {
        let instruments = indexed_instruments();
        let kraken = generate_execution_instrument_map(&instruments, ExchangeId::Kraken).unwrap();
        let usdc_usdt = test_utils::instrument(ExchangeId::Kraken, "USDC", "USDT");

        let usdc_usdt_index = instruments
            .find_instrument_index(ExchangeId::Kraken, &usdc_usdt.name_internal)
            .unwrap();

        let usdc_usdt_exchange_name = kraken
            .find_instrument_name_exchange(usdc_usdt_index)
            .unwrap();

        assert_eq!(usdc_usdt_exchange_name, &usdc_usdt.name_exchange);
    }

    #[test]
    fn test_find_instrument_index() {
        let instruments = indexed_instruments();
        let kraken = generate_execution_instrument_map(&instruments, ExchangeId::Kraken).unwrap();

        let usdc_usdt = test_utils::instrument(ExchangeId::Kraken, "USDC", "USDT");
        let instrument_index = kraken
            .find_instrument_index(&usdc_usdt.name_exchange)
            .unwrap();

        let expected_index = instruments
            .find_instrument_index(ExchangeId::Kraken, &usdc_usdt.name_internal)
            .unwrap();
        assert_eq!(instrument_index, expected_index);
    }

    #[test]
    fn test_find_instrument_index_error() {
        let instruments = indexed_instruments();
        let kraken = generate_execution_instrument_map(&instruments, ExchangeId::Kraken).unwrap();

        let btc_eth = test_utils::instrument(ExchangeId::Kraken, "BTC", "ETH");
        let result = kraken.find_instrument_index(&btc_eth.name_exchange);
        assert!(result.is_err());
        assert!(matches!(result, Err(IndexError::InstrumentIndex(_))));
    }

    #[test]
    fn test_find_instrument_name_exchange_error() {
        let instruments = indexed_instruments();
        let kraken = generate_execution_instrument_map(&instruments, ExchangeId::Kraken).unwrap();

        // Try to find instrument with invalid index
        let invalid_index = InstrumentIndex::new(999);
        let result = kraken.find_instrument_name_exchange(invalid_index);
        assert!(result.is_err());
        assert!(matches!(result, Err(KeyError::InstrumentKey(_))));
    }

    #[test]
    fn test_exchange_assets_iterator() {
        let instruments = indexed_instruments();
        let kraken = generate_execution_instrument_map(&instruments, ExchangeId::Kraken).unwrap();

        let exchange_assets: Vec<&AssetNameExchange> = kraken.exchange_assets().collect();

        // Verify that the iterator returns the expected assets for Kraken
        let expected_assets = vec!["USDC", "USDT"];
        for expected in &expected_assets {
            assert!(
                exchange_assets
                    .iter()
                    .any(|asset| asset.as_ref() == *expected)
            );
        }
    }

    #[test]
    fn test_exchange_instruments_iterator() {
        let instruments = indexed_instruments();
        let kraken = generate_execution_instrument_map(&instruments, ExchangeId::Kraken).unwrap();

        let exchange_instruments: Vec<&InstrumentNameExchange> =
            kraken.exchange_instruments().collect();

        // Should have exactly one instrument for Kraken
        assert_eq!(exchange_instruments.len(), 1);

        // Verify it contains the USDC-USDT instrument
        let usdc_usdt = test_utils::instrument(ExchangeId::Kraken, "USDC", "USDT");
        assert!(
            exchange_instruments
                .iter()
                .any(|instr| *instr == &usdc_usdt.name_exchange)
        );
    }

    #[test]
    fn test_generate_execution_instrument_map_error() {
        let instruments = indexed_instruments();

        // Try to generate map for exchange not in indexed instruments
        let result = generate_execution_instrument_map(&instruments, ExchangeId::Bitstamp);
        assert!(result.is_err());
        assert!(matches!(result, Err(IndexError::ExchangeIndex(_))));
    }
}
