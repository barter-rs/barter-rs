use crate::{asset::name::AssetNameExchange, exchange::ExchangeId};
use derive_more::Display;
use serde::Serialize;
use smol_str::{SmolStr, StrExt, format_smolstr};
use std::borrow::Borrow;

/// Barter lowercase `SmolStr` representation for an [`Instrument`](super::Instrument) - unique
/// across all exchanges.
///
/// Note: Binance btc_usdt spot is not considered the same instrument as Bitfinex btc_usdt spot.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Display)]
pub struct InstrumentNameInternal(pub SmolStr);

impl InstrumentNameInternal {
    /// Construct a new lowercase [`Self`] from the provided `Into<SmolStr>`.
    ///
    /// Should be unique across exchanges.
    pub fn new<S>(name: S) -> Self
    where
        S: Into<SmolStr>,
    {
        let name = name.into();
        if name.chars().all(char::is_lowercase) {
            Self(name)
        } else {
            Self(name.to_lowercase_smolstr())
        }
    }

    /// Construct a new lowercase [`Self`], combining the [`ExchangeId`] and
    /// base and quote [`AssetNameExchange`]s.
    ///
    /// Generates an internal instrument identifier unique across exchanges.
    pub fn new_from_exchange_underlying<Ass>(exchange: ExchangeId, base: &Ass, quote: &Ass) -> Self
    where
        for<'a> &'a Ass: Into<&'a AssetNameExchange>,
    {
        Self::new(format_smolstr!(
            "{exchange}-{}_{}",
            base.into(),
            quote.into()
        ))
    }

    /// Construct a new lowercase [`Self`], combining the [`ExchangeId`] and
    /// [`InstrumentNameExchange`].
    ///
    /// Generates an internal instrument identifier unique across exchanges.
    pub fn new_from_exchange<S>(exchange: ExchangeId, name_exchange: S) -> Self
    where
        S: Into<InstrumentNameExchange>,
    {
        let name_exchange = name_exchange.into();
        let exchange = exchange.as_str();
        Self::new(format_smolstr!("{exchange}-{name_exchange}"))
    }

    /// Return the internal instrument `SmolStr` name of [`Self`].
    pub fn name(&self) -> &SmolStr {
        &self.0
    }
}

impl From<&str> for InstrumentNameInternal {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<SmolStr> for InstrumentNameInternal {
    fn from(value: SmolStr) -> Self {
        Self::new(value)
    }
}

impl From<String> for InstrumentNameInternal {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl Borrow<str> for InstrumentNameInternal {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl AsRef<str> for InstrumentNameInternal {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<'de> serde::de::Deserialize<'de> for InstrumentNameInternal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let name = std::borrow::Cow::<'de, str>::deserialize(deserializer)?;
        Ok(InstrumentNameInternal::new(name))
    }
}

/// Exchange `SmolStr` representation for an [`Instrument`](super::Instrument) - most likely not
/// unique across all exchanges.
///
/// For example: `InstrumentNameExchange("XBT-USDT")`, which is distinct from the internal
/// representation of the instrument, such as `InstrumentIndex(1)` or
/// `InstrumentNameInternal("btc_usdt"`.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Display)]
pub struct InstrumentNameExchange(SmolStr);

impl InstrumentNameExchange {
    /// Construct a new [`Self`] from the provided `Into<SmolStr>`.
    pub fn new<S>(name: S) -> Self
    where
        S: Into<SmolStr>,
    {
        Self(name.into())
    }

    /// Return the execution instrument `SmolStr` name of [`Self`].
    pub fn name(&self) -> &SmolStr {
        &self.0
    }
}

impl From<&str> for InstrumentNameExchange {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<SmolStr> for InstrumentNameExchange {
    fn from(value: SmolStr) -> Self {
        Self::new(value)
    }
}

impl From<String> for InstrumentNameExchange {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl Borrow<str> for InstrumentNameExchange {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl AsRef<str> for InstrumentNameExchange {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<'de> serde::de::Deserialize<'de> for InstrumentNameExchange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let name = std::borrow::Cow::<'de, str>::deserialize(deserializer)?;
        Ok(InstrumentNameExchange::new(name))
    }
}
