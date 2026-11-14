use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// `FutureContract` specification containing all the information needed to fully identify a
/// future instrument.
///
/// # Type Parameters
/// * `AssetKey` - Type used to identify the settlement asset for the option contract.
///
/// # Fields
/// * `contract_size` - Multiplier that determines how many of the underlying asset the contract represents.
/// * `settlement_asset` - Asset used for settlement when the future expires.
/// * `expiry` - The date and time when the future expires.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct FutureContract<AssetKey> {
    pub contract_size: Decimal,
    pub settlement_asset: AssetKey,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub expiry: DateTime<Utc>,
}
