use crate::{
    Keyed,
    asset::{Asset, AssetIndex, ExchangeAsset, name::AssetNameInternal},
    exchange::{ExchangeId, ExchangeIndex},
    index::{builder::IndexedInstrumentsBuilder, error::IndexError},
    instrument::{Instrument, InstrumentIndex, name::InstrumentNameInternal},
};
use serde::{Deserialize, Serialize};

pub mod builder;

/// Contains error variants that can occur when working with an [`IndexedInstruments`] collection.
pub mod error;

/// Indexed collection of exchanges, assets, and instruments.
///
/// Initialise incrementally via the [`IndexedInstrumentsBuilder`], or all at once via the
/// constructor.
///
/// The indexed collection is useful for creating efficient O(1) constant lookup state management
/// systems where the state is keyed on an instrument, asset, or exchange.
///
/// For example uses cases, see the central `barter` crate `EngineState` design.
///
/// # Index Relationships
/// - `ExchangeIndex`: Unique index for each [`ExchangeId`] added during initialisation.
/// - `InstrumentIndex`: Unique identifier for each [`Instrument`] added during initialisation.
/// - `AssetIndex`: Unique identifier for each [`ExchangeAsset`] added during initialisation.
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct IndexedInstruments {
    exchanges: Vec<Keyed<ExchangeIndex, ExchangeId>>,
    assets: Vec<Keyed<AssetIndex, ExchangeAsset<Asset>>>,
    instruments:
        Vec<Keyed<InstrumentIndex, Instrument<Keyed<ExchangeIndex, ExchangeId>, AssetIndex>>>,
}

impl IndexedInstruments {
    /// Initialises a new `IndexedInstruments` from an iterator of [`Instrument`]s.
    ///
    /// This method indexes all unique exchanges, assets, and instruments, creating efficient
    /// lookup tables for each entity type.
    ///
    /// Note that once an `IndexedInstruments` has been constructed, it cannot be mutated (this
    /// could invalidate existing index lookup tables).
    ///
    /// For incremental initialisation, see the [`IndexedInstrumentsBuilder`].
    pub fn new<Iter, I>(instruments: Iter) -> Self
    where
        Iter: IntoIterator<Item = I>,
        I: Into<Instrument<ExchangeId, Asset>>,
    {
        instruments
            .into_iter()
            .fold(Self::builder(), |builder, instrument| {
                builder.add_instrument(instrument.into())
            })
            .build()
    }

    /// Returns a new [`IndexedInstrumentsBuilder`] useful for incremental initialisation of
    /// `IndexedInstruments`.
    pub fn builder() -> IndexedInstrumentsBuilder {
        IndexedInstrumentsBuilder::default()
    }

    /// Returns a reference to the [`ExchangeIndex`] <--> [`ExchangeId`] associations.
    pub fn exchanges(&self) -> &[Keyed<ExchangeIndex, ExchangeId>] {
        &self.exchanges
    }

    /// Returns a reference to the [`AssetIndex`] <--> [`ExchangeAsset`] associations.
    pub fn assets(&self) -> &[Keyed<AssetIndex, ExchangeAsset<Asset>>] {
        &self.assets
    }

    /// Returns a reference to the [`InstrumentIndex`] <--> [`Instrument`] associations.
    pub fn instruments(
        &self,
    ) -> &[Keyed<InstrumentIndex, Instrument<Keyed<ExchangeIndex, ExchangeId>, AssetIndex>>] {
        &self.instruments
    }

    /// Finds the [`ExchangeIndex`] associated with the provided [`ExchangeId`].
    ///
    /// # Arguments
    /// * `exchange` - The exchange ID to look up
    ///
    /// # Returns
    /// * `Ok(ExchangeIndex)` - exchange found.
    /// * `Err(IndexError)` - exchange not found.
    pub fn find_exchange_index(&self, exchange: ExchangeId) -> Result<ExchangeIndex, IndexError> {
        find_exchange_by_exchange_id(&self.exchanges, &exchange)
    }

    pub fn find_exchange(&self, index: ExchangeIndex) -> Result<ExchangeId, IndexError> {
        self.exchanges
            .iter()
            .find(|keyed| keyed.key == index)
            .map(|keyed| keyed.value)
            .ok_or(IndexError::ExchangeIndex(format!(
                "ExchangeIndex: {index} is not present in indexed instrument exchanges"
            )))
    }

    /// Finds the [`AssetIndex`] associated with the provided `ExchangeId` and `AssetNameInterval`.
    ///
    /// # Arguments
    /// * `exchange` - The `ExchangeId` associated with the asset.
    /// * `name` - The `AssetNameInternal` associated with the asset (eg/ "btc", "usdt", etc).
    ///
    /// # Returns
    /// * `Ok(AssetIndex)` - exchange asset found.
    /// * `Err(IndexError)` - exchange asset not found.
    pub fn find_asset_index(
        &self,
        exchange: ExchangeId,
        name: &AssetNameInternal,
    ) -> Result<AssetIndex, IndexError> {
        find_asset_by_exchange_and_name_internal(&self.assets, exchange, name)
    }

    pub fn find_asset(&self, index: AssetIndex) -> Result<&ExchangeAsset<Asset>, IndexError> {
        self.assets
            .iter()
            .find(|keyed| keyed.key == index)
            .map(|keyed| &keyed.value)
            .ok_or(IndexError::AssetIndex(format!(
                "AssetIndex: {index} is not present in indexed instrument assets"
            )))
    }

    /// Finds the [`InstrumentIndex`] associated with the provided `ExchangeId` and
    /// `InstrumentNameInternal`.
    ///
    /// # Arguments
    /// * `exchange` - The `ExchangeId` associated with the instrument.
    /// * `name` - The `InstrumentNameInternal` associated with the instrument (eg/ binance_spot_btc_usdt).
    ///
    /// # Returns
    /// * `Ok(AssetIndex)` - instrument found.
    /// * `Err(IndexError)` - instrument not found.
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
                "Asset: ({}, {}) is not present in indexed instrument assets: {:?}",
                exchange, name, self.assets
            )))
    }

    pub fn find_instrument(
        &self,
        index: InstrumentIndex,
    ) -> Result<&Instrument<Keyed<ExchangeIndex, ExchangeId>, AssetIndex>, IndexError> {
        self.instruments
            .iter()
            .find(|keyed| keyed.key == index)
            .map(|keyed| &keyed.value)
            .ok_or(IndexError::InstrumentIndex(format!(
                "InstrumentIndex: {index} is not present in indexed instrument instruments"
            )))
    }
}

impl<I> FromIterator<I> for IndexedInstruments
where
    I: Into<Instrument<ExchangeId, Asset>>,
{
    fn from_iter<Iter>(iter: Iter) -> Self
    where
        Iter: IntoIterator<Item = I>,
    {
        Self::new(iter)
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
            "Exchange: {needle} is not present in indexed instrument exchanges: {haystack:?}"
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
            "Asset: ({needle_exchange}, {needle_name}) is not present in indexed instrument assets: {haystack:?}"
        )))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        Underlying,
        asset::Asset,
        exchange::ExchangeId,
        instrument::{
            kind::InstrumentKind, name::InstrumentNameExchange, quote::InstrumentQuoteAsset,
        },
        test_utils::{exchange_asset, instrument},
    };

    #[test]
    fn test_indexed_instruments_new() {
        // Test creating empty IndexedInstruments
        let empty = IndexedInstruments::new(std::iter::empty::<Instrument<ExchangeId, Asset>>());
        assert!(empty.exchanges().is_empty());
        assert!(empty.assets().is_empty());
        assert!(empty.instruments().is_empty());

        // Test creating with single instrument
        let instrument = instrument(ExchangeId::BinanceSpot, "btc", "usdt");
        let actual = IndexedInstruments::new(std::iter::once(instrument));

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
                quote: InstrumentQuoteAsset::UnderlyingQuote,
                kind: InstrumentKind::Spot,
                spec: None
            }
        );
    }

    #[test]
    fn test_indexed_instruments_multiple() {
        let instruments = vec![
            instrument(ExchangeId::BinanceSpot, "BTC", "USDT"),
            instrument(ExchangeId::BinanceSpot, "ETH", "USDT"),
            instrument(ExchangeId::Coinbase, "BTC", "USD"),
        ];

        let indexed = IndexedInstruments::new(instruments);

        // Should have 2 exchanges, 4 assets (BTC, ETH, USDT, USD), and 3 instruments
        assert_eq!(indexed.exchanges().len(), 2);
        assert_eq!(indexed.assets().len(), 5);
        assert_eq!(indexed.instruments().len(), 3);

        // Verify exchanges
        let exchanges: Vec<_> = indexed.exchanges().iter().map(|e| e.value).collect();
        assert!(exchanges.contains(&ExchangeId::BinanceSpot));
        assert!(exchanges.contains(&ExchangeId::Coinbase));
    }

    #[test]
    fn test_find_exchange_index() {
        let instruments = vec![
            instrument(ExchangeId::BinanceSpot, "BTC", "USDT"),
            instrument(ExchangeId::Coinbase, "ETH", "USD"),
        ];
        let indexed = IndexedInstruments::new(instruments);

        // Test finding existing exchanges
        assert!(indexed.find_exchange_index(ExchangeId::BinanceSpot).is_ok());
        assert!(indexed.find_exchange_index(ExchangeId::Coinbase).is_ok());

        // Test finding non-existent exchange
        let err = indexed.find_exchange_index(ExchangeId::Kraken).unwrap_err();
        assert!(matches!(err, IndexError::ExchangeIndex(_)));
    }

    #[test]
    fn test_find_asset_index() {
        let instruments = vec![
            instrument(ExchangeId::BinanceSpot, "BTC", "USDT"),
            instrument(ExchangeId::Coinbase, "ETH", "USD"),
        ];
        let indexed = IndexedInstruments::new(instruments);

        // Test finding existing assets
        assert!(
            indexed
                .find_asset_index(ExchangeId::BinanceSpot, &AssetNameInternal::from("btc"))
                .is_ok()
        );
        assert!(
            indexed
                .find_asset_index(ExchangeId::BinanceSpot, &AssetNameInternal::from("usdt"))
                .is_ok()
        );
        assert!(
            indexed
                .find_asset_index(ExchangeId::Coinbase, &AssetNameInternal::from("eth"))
                .is_ok()
        );

        // Test finding asset with wrong exchange
        let err = indexed
            .find_asset_index(ExchangeId::Kraken, &AssetNameInternal::from("btc"))
            .unwrap_err();
        assert!(matches!(err, IndexError::AssetIndex(_)));

        // Test finding non-existent asset
        let err = indexed
            .find_asset_index(
                ExchangeId::BinanceSpot,
                &AssetNameInternal::from("nonexistent"),
            )
            .unwrap_err();
        assert!(matches!(err, IndexError::AssetIndex(_)));
    }

    #[test]
    fn test_find_instrument_index() {
        let instruments = vec![
            instrument(ExchangeId::BinanceSpot, "btc", "usdt"),
            instrument(ExchangeId::Coinbase, "eth", "usd"),
        ];

        let indexed = IndexedInstruments::new(instruments);
        let btc_usdt = InstrumentNameInternal::from("binance_spot-btc_usdt");

        // Test finding existing instruments
        assert!(
            indexed
                .find_instrument_index(ExchangeId::BinanceSpot, &btc_usdt)
                .is_ok()
        );

        // Test finding instrument with wrong exchange
        let err = indexed
            .find_instrument_index(ExchangeId::Kraken, &btc_usdt)
            .unwrap_err();
        assert!(matches!(err, IndexError::AssetIndex(_)));

        // Test finding non-existent instrument
        let nonexistent = InstrumentNameInternal::from("nonexistent");
        let err = indexed
            .find_instrument_index(ExchangeId::BinanceSpot, &nonexistent)
            .unwrap_err();
        assert!(matches!(err, IndexError::AssetIndex(_)));
    }

    #[test]
    fn test_private_find_exchange_by_exchange_id() {
        let exchanges = vec![
            Keyed {
                key: ExchangeIndex(0),
                value: ExchangeId::BinanceSpot,
            },
            Keyed {
                key: ExchangeIndex(1),
                value: ExchangeId::Coinbase,
            },
        ];

        // Test finding existing exchange
        let result = find_exchange_by_exchange_id(&exchanges, &ExchangeId::BinanceSpot);
        assert_eq!(result.unwrap(), ExchangeIndex(0));

        // Test finding non-existent exchange
        let err = find_exchange_by_exchange_id(&exchanges, &ExchangeId::Kraken).unwrap_err();
        assert!(matches!(err, IndexError::ExchangeIndex(_)));
    }

    #[test]
    fn test_private_find_asset_by_exchange_and_name_internal() {
        let assets = vec![
            Keyed {
                key: AssetIndex(0),
                value: ExchangeAsset {
                    exchange: ExchangeId::BinanceSpot,
                    asset: Asset::new_from_exchange("BTC"),
                },
            },
            Keyed {
                key: AssetIndex(1),
                value: ExchangeAsset {
                    exchange: ExchangeId::BinanceSpot,
                    asset: Asset::new_from_exchange("USDT"),
                },
            },
        ];

        // Test finding existing asset
        let result = find_asset_by_exchange_and_name_internal(
            &assets,
            ExchangeId::BinanceSpot,
            &AssetNameInternal::from("btc"),
        );
        assert_eq!(result.unwrap(), AssetIndex(0));

        // Test finding asset with wrong exchange
        let err = find_asset_by_exchange_and_name_internal(
            &assets,
            ExchangeId::Kraken,
            &AssetNameInternal::from("btc"),
        )
        .unwrap_err();
        assert!(matches!(err, IndexError::AssetIndex(_)));

        // Test finding non-existent asset
        let err = find_asset_by_exchange_and_name_internal(
            &assets,
            ExchangeId::BinanceSpot,
            &AssetNameInternal::from("nonexistent"),
        )
        .unwrap_err();
        assert!(matches!(err, IndexError::AssetIndex(_)));
    }

    #[test]
    fn test_duplicates_are_filtered_correctly() {
        // Test with duplicate instruments
        let instruments = vec![
            instrument(ExchangeId::BinanceSpot, "btc", "usdt"),
            instrument(ExchangeId::BinanceSpot, "btc", "usdt"),
        ];
        let indexed = IndexedInstruments::new(instruments);

        // Should deduplicate exchanges and assets
        assert_eq!(indexed.exchanges().len(), 1);
        assert_eq!(indexed.assets().len(), 2);
        assert_eq!(indexed.instruments().len(), 1); // Instruments aren't deduplicated

        // Test with same asset on different exchanges
        let instruments = vec![
            instrument(ExchangeId::BinanceSpot, "btc", "usdt"),
            instrument(ExchangeId::Coinbase, "btc", "usdt"),
        ];
        let indexed = IndexedInstruments::new(instruments);

        // Should have separate entries for same asset on different exchanges
        assert_eq!(indexed.exchanges().len(), 2);
        assert_eq!(indexed.assets().len(), 4); // BTC and USDT on both exchanges
        assert_eq!(indexed.instruments().len(), 2);
    }
}
