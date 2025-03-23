use derive_more::Display;
use serde::Serialize;
use smol_str::{SmolStr, StrExt};
use std::borrow::Borrow;

/// Barter lowercase `SmolStr` representation for an [`Asset`](super::Asset) - not unique across
/// exchanges.
///
/// This may or may not be different from an execution's representation.
///
/// For example, some exchanges may refer to "btc" as "xbt".
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Display)]
pub struct AssetNameInternal(SmolStr);

impl AssetNameInternal {
    /// Construct a new lowercase [`Self`] from the provided `Into<SmolStr>`.
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

    /// Return the internal asset `SmolStr` name of [`Self`].
    pub fn name(&self) -> &SmolStr {
        &self.0
    }
}

impl From<&str> for AssetNameInternal {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<SmolStr> for AssetNameInternal {
    fn from(value: SmolStr) -> Self {
        Self::new(value)
    }
}

impl From<String> for AssetNameInternal {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl Borrow<str> for AssetNameInternal {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl AsRef<str> for AssetNameInternal {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<'de> serde::de::Deserialize<'de> for AssetNameInternal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let name = std::borrow::Cow::<'de, str>::deserialize(deserializer)?;
        Ok(AssetNameInternal::new(name))
    }
}

/// Exchange `SmolStr` representation for an [`Asset`](super::Asset) - not unique across exchanges.
///
/// For example: `AssetNameExchange("XBT")`, which is distinct from the internal representation
/// of the asset, such as `AssetIndex(1)` or `AssetNameInternal("btc")`.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Display)]
pub struct AssetNameExchange(SmolStr);

impl AssetNameExchange {
    /// Construct a new [`Self`] from the provided `Into<SmolStr>`.
    pub fn new<S>(name: S) -> Self
    where
        S: Into<SmolStr>,
    {
        Self(name.into())
    }

    /// Return the execution asset `SmolStr` name of [`Self`].
    pub fn name(&self) -> &SmolStr {
        &self.0
    }
}

impl From<&str> for AssetNameExchange {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<SmolStr> for AssetNameExchange {
    fn from(value: SmolStr) -> Self {
        Self::new(value)
    }
}

impl From<String> for AssetNameExchange {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl Borrow<str> for AssetNameExchange {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl AsRef<str> for AssetNameExchange {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<'de> serde::de::Deserialize<'de> for AssetNameExchange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let name = std::borrow::Cow::<'de, str>::deserialize(deserializer)?;
        Ok(AssetNameExchange::new(name))
    }
}
