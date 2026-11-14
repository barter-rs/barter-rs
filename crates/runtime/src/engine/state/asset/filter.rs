use serde::{Deserialize, Serialize};
use trading_bot_instrument::exchange::ExchangeId;
use trading_bot_integration::collection::one_or_many::OneOrMany;

/// Asset filter.
///
/// Used to filter asset-centric data structures such as `AssetStates`.
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub enum AssetFilter {
    None,
    Exchanges(OneOrMany<ExchangeId>),
}

impl AssetFilter {
    pub fn exchanges(exchanges: impl IntoIterator<Item = ExchangeId>) -> Self {
        Self::Exchanges(OneOrMany::from_iter(exchanges))
    }
}
