use serde::{Deserialize, Deserializer, Serialize};
use smol_str::{SmolStr, StrExt};
use std::fmt::{Display, Formatter};

/// Barter new type representing a currency symbol `String` identifier.
///
/// eg/ "btc", "eth", "usdt", etc
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub struct Symbol(SmolStr);

impl Display for Symbol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Symbol {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Symbol::new)
    }
}

impl<S> From<S> for Symbol
where
    S: Into<SmolStr>,
{
    fn from(input: S) -> Self {
        Symbol::new(input)
    }
}

impl Symbol {
    /// Construct a new [`Symbol`] new type using the provided `Into<Symbol>` value.
    pub fn new<S>(input: S) -> Self
    where
        S: Into<SmolStr>,
    {
        Self(input.into().to_lowercase_smolstr())
    }
}
