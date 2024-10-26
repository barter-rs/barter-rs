use crate::exchange::ExchangeId;
use derive_more::{Display, From};
use serde::Serialize;
use smol_str::{format_smolstr, SmolStr, StrExt};
use std::borrow::Borrow;

/// Barter `SmolStr` representation for an [`Instrument`](super::Instrument) - unique across
/// all exchanges.
///
/// Note: Binance btc_usdt spot is not considered the same instrument as Bitfinex btc_usdt spot.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Display)]
pub struct InstrumentNameInternal(pub SmolStr);

impl InstrumentNameInternal {
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

    pub fn new_from_exchange<S>(exchange: ExchangeId, name_exchange: S) -> Self
    where
        S: Into<SmolStr>,
    {
        let name_exchange = name_exchange.into();
        let exchange = exchange.as_str();
        Self::new(format_smolstr!("{exchange}-{name_exchange}"))
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
        let name = <&str>::deserialize(deserializer)?;
        Ok(InstrumentNameInternal::new(name))
    }
}
