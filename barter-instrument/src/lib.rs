#![forbid(unsafe_code)]
#![warn(
    unused,
    clippy::cognitive_complexity,
    unused_crate_dependencies,
    unused_extern_crates,
    clippy::unused_self,
    clippy::useless_let_if_seq,
    missing_debug_implementations,
    rust_2018_idioms,
    rust_2024_compatibility
)]
#![allow(clippy::type_complexity, clippy::too_many_arguments, type_alias_bounds)]

//! # Barter-Instrument
//! Barter-Instrument contains core Exchange, Instrument and Asset data structures and associated utilities.
//!
//! ## Examples
//! For a comprehensive collection of examples, see the Barter core Engine /examples directory.

use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Defines a global [`ExchangeId`](exchange::ExchangeId) enum covering all exchanges.
pub mod exchange;

/// [`Asset`](asset::Asset) related data structures.
///
/// eg/ `AssetKind`, `AssetNameInternal`, etc.
pub mod asset;

/// [`Instrument`](instrument::Instrument) related data structures.
///
/// eg/ `InstrumentKind`, `OptionContract``, etc.
pub mod instrument;

/// Indexed collection of exchanges, assets, and instruments. Provides a builder utility for
/// indexing non-indexed collections.
pub mod index;

/// A keyed value.
///
/// eg/ Keyed<InstrumentIndex, Instrument>
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct Keyed<Key, Value> {
    pub key: Key,
    pub value: Value,
}

impl<Key, Value> AsRef<Value> for Keyed<Key, Value> {
    fn as_ref(&self) -> &Value {
        &self.value
    }
}

impl<Key, Value> Display for Keyed<Key, Value>
where
    Key: Display,
    Value: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}", self.key, self.value)
    }
}

/// Instrument Underlying containing a base and quote asset.
///
/// eg/ Underlying { base: "btc", quote: "usdt" }
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Underlying<AssetKey> {
    pub base: AssetKey,
    pub quote: AssetKey,
}

impl<AssetKey> Underlying<AssetKey> {
    pub fn new<A>(base: A, quote: A) -> Self
    where
        A: Into<AssetKey>,
    {
        Self {
            base: base.into(),
            quote: quote.into(),
        }
    }
}

/// [`Side`] of a trade or position - Buy or Sell.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum Side {
    #[serde(alias = "buy", alias = "BUY", alias = "b")]
    Buy,
    #[serde(alias = "sell", alias = "SELL", alias = "s")]
    Sell,
}

impl Display for Side {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Side::Buy => "buy",
                Side::Sell => "sell",
            }
        )
    }
}

pub mod test_utils {
    use crate::{
        Underlying,
        asset::{
            Asset, ExchangeAsset,
            name::{AssetNameExchange, AssetNameInternal},
        },
        exchange::ExchangeId,
        instrument::{
            Instrument,
            kind::InstrumentKind,
            name::{InstrumentNameExchange, InstrumentNameInternal},
            quote::InstrumentQuoteAsset,
        },
    };

    pub fn exchange_asset(exchange: ExchangeId, symbol: &str) -> ExchangeAsset<Asset> {
        ExchangeAsset {
            exchange,
            asset: asset(symbol),
        }
    }

    pub fn asset(symbol: &str) -> Asset {
        Asset {
            name_internal: AssetNameInternal::from(symbol),
            name_exchange: AssetNameExchange::from(symbol),
        }
    }

    pub fn instrument(
        exchange: ExchangeId,
        base: &str,
        quote: &str,
    ) -> Instrument<ExchangeId, Asset> {
        let name_exchange = InstrumentNameExchange::from(format!("{base}_{quote}"));
        let name_internal =
            InstrumentNameInternal::new_from_exchange(exchange, name_exchange.clone());
        let base_asset = asset(base);
        let quote_asset = asset(quote);

        Instrument::new(
            exchange,
            name_internal,
            name_exchange,
            Underlying::new(base_asset, quote_asset),
            InstrumentQuoteAsset::UnderlyingQuote,
            InstrumentKind::Spot,
            None,
        )
    }
}
