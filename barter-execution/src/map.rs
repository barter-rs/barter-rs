use crate::error::KeyError;
use barter_instrument::{
    Keyed,
    asset::{AssetIndex, name::AssetNameExchange},
    exchange::{ExchangeId, ExchangeIndex},
    index::{IndexedInstruments, error::IndexError},
    instrument::{InstrumentIndex, name::InstrumentNameExchange},
};
use barter_integration::collection::FnvIndexMap;
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
    pub exchange: Keyed<ExchangeIndex, ExchangeId>,
    pub assets: FnvIndexMap<AssetIndex, AssetNameExchange>,
    pub instruments: FnvIndexMap<InstrumentIndex, InstrumentNameExchange>,
    pub asset_names: FnvHashMap<AssetNameExchange, AssetIndex>,
    pub instrument_names: FnvHashMap<InstrumentNameExchange, InstrumentIndex>,
}

impl ExecutionInstrumentMap {
    /// Construct a new [`Self`] using the provided indexed assets and instruments.
    pub fn new(
        exchange: Keyed<ExchangeIndex, ExchangeId>,
        assets: FnvIndexMap<AssetIndex, AssetNameExchange>,
        instruments: FnvIndexMap<InstrumentIndex, InstrumentNameExchange>,
    ) -> Self {
        Self {
            exchange,
            asset_names: assets
                .iter()
                .map(|(key, value)| (value.clone(), *key))
                .collect(),
            instrument_names: instruments
                .iter()
                .map(|(key, value)| (value.clone(), *key))
                .collect(),
            assets: assets,
            instruments: instruments,
        }
    }

    pub fn exchange_assets(&self) -> impl Iterator<Item = &AssetNameExchange> {
        self.assets.values()
    }

    pub fn exchange_instruments(&self) -> impl Iterator<Item = &InstrumentNameExchange> {
        self.instruments.values()
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
        self.assets.get(&asset).ok_or_else(|| {
            KeyError::AssetKey(format!("ExecutionInstrumentMap does not contain: {asset}"))
        })
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
        self.instruments.get(&instrument).ok_or_else(|| {
            KeyError::InstrumentKey(format!(
                "ExecutionInstrumentMap does not contain: {instrument}"
            ))
        })
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
        instruments
            .assets()
            .iter()
            .filter_map(|asset| {
                (asset.value.exchange == exchange)
                    .then_some((asset.key, asset.value.asset.name_exchange.clone()))
            })
            .collect(),
        instruments
            .instruments()
            .iter()
            .filter_map(|instrument| {
                (instrument.value.exchange.value == exchange)
                    .then_some((instrument.key, instrument.value.name_exchange.clone()))
            })
            .collect(),
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
}
