use derive_more::{Display, From};
use serde::Serialize;
use smol_str::{SmolStr, StrExt};
use std::borrow::Borrow;

/// Barter `SmolStr` representation for an [`Asset`](super::Asset) - not unique across exchanges.
///
/// This may or may not be different from an exchange's representation.
///
/// For example, some exchanges may refer to "btc" as "xbt".
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Display)]
pub struct AssetNameInternal(SmolStr);

impl AssetNameInternal {
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
        let name = <&str>::deserialize(deserializer)?;
        Ok(AssetNameInternal::new(name))
    }
}
