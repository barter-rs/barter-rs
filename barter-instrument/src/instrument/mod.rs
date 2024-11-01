use crate::{
    asset::Asset,
    exchange::ExchangeId,
    instrument::{
        kind::InstrumentKind,
        market_data::{kind::MarketDataInstrumentKind, MarketDataInstrument},
        name::InstrumentNameInternal,
        spec::{InstrumentSpec, InstrumentSpecQuantity, OrderQuantityUnits},
    },
    Underlying,
};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

pub mod kind;

/// Defines the Barter [`AssetNameInternal`], used as a `SmolStr` identifier for an [`Asset`]
/// (not unique across exchanges).
pub mod name;

/// Defines the [`InstrumentSpec`], including specifications for an [`Instrument`]s
/// price, quantity and notional value.
///
/// eg/ `InstrumentSpecPrice.tick_size`, `OrderQuantityUnits`, etc.  
pub mod spec;

/// Defines a simplified [`MarketDataInstrument`], with only the necessary data to subscribe to
/// market data feeds.  
pub mod market_data;

/// Unique identifier for an `Instrument` traded on an exchange.
///
/// Used to key data events in a memory efficient way.
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display,
)]
pub struct InstrumentId(pub u64);

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display,
)]
pub struct InstrumentIndex(usize);

/// Comprehensive Instrument model, containing all the data required to subscribe to market data
/// and generate correct orders.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Instrument<AssetKey> {
    pub exchange: ExchangeId,
    pub name_internal: InstrumentNameInternal,
    pub name_exchange: SmolStr,
    pub underlying: Underlying<AssetKey>,
    #[serde(alias = "instrument_kind")]
    pub kind: InstrumentKind<AssetKey>,
    pub spec: InstrumentSpec<AssetKey>,
}

impl<AssetKey> Instrument<AssetKey> {
    /// Construct a new [`Self`] with the provided data, assuming the [`InstrumentNameInternal`]
    /// can be created via the [`InstrumentNameInternal::new_from_exchange`] constructor.
    pub fn new<NameExchange>(
        exchange: ExchangeId,
        name_exchange: NameExchange,
        underlying: Underlying<AssetKey>,
        kind: InstrumentKind<AssetKey>,
        spec: InstrumentSpec<AssetKey>,
    ) -> Self
    where
        NameExchange: Into<SmolStr>,
    {
        let name_exchange = name_exchange.into();
        let name_internal =
            InstrumentNameInternal::new_from_exchange(exchange, name_exchange.clone());

        Self {
            exchange,
            name_internal,
            name_exchange,
            underlying,
            kind,
            spec,
        }
    }

    /// Map this Instruments `AssetKey` to a new key, using the provided lookup closure.
    pub fn map_asset_key<FnFindAsset, NewAssetKey, Error>(
        self,
        find_asset: FnFindAsset,
    ) -> Result<Instrument<NewAssetKey>, Error>
    where
        FnFindAsset: Fn(&AssetKey) -> Result<NewAssetKey, Error>,
    {
        let Instrument {
            exchange,
            name_internal,
            name_exchange,
            underlying: Underlying { base, quote },
            kind,
            spec:
                InstrumentSpec {
                    price,
                    quantity:
                        InstrumentSpecQuantity {
                            unit,
                            min,
                            increment,
                        },
                    notional,
                },
        } = self;

        let base_index = find_asset(&base)?;
        let quote_index = find_asset(&quote)?;

        let kind = match kind {
            InstrumentKind::Spot => InstrumentKind::Spot,
            InstrumentKind::Perpetual { settlement_asset } => InstrumentKind::Perpetual {
                settlement_asset: find_asset(&settlement_asset)?,
            },
            InstrumentKind::Future {
                settlement_asset,
                contract,
            } => InstrumentKind::Future {
                settlement_asset: find_asset(&settlement_asset)?,
                contract,
            },
            InstrumentKind::Option {
                settlement_asset,
                contract,
            } => InstrumentKind::Option {
                settlement_asset: find_asset(&settlement_asset)?,
                contract,
            },
        };
        let unit = match unit {
            OrderQuantityUnits::Asset(asset) => OrderQuantityUnits::Asset(find_asset(&asset)?),
            OrderQuantityUnits::Contract => OrderQuantityUnits::Contract,
            OrderQuantityUnits::Quote => OrderQuantityUnits::Quote,
        };

        Ok(Instrument {
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
    }
}

impl From<&Instrument<Asset>> for MarketDataInstrument {
    fn from(value: &Instrument<Asset>) -> Self {
        Self {
            base: value.underlying.base.name_internal.clone(),
            quote: value.underlying.quote.name_internal.clone(),
            kind: MarketDataInstrumentKind::from(&value.kind),
        }
    }
}
