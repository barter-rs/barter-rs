use rust_decimal::Decimal;
/// `PerpetualContract` specification containing all the information needed to fully identify a
/// perpetual instrument.
///
/// # Type Parameters
/// * `AssetKey` - Type used to identify the settlement asset for the option contract.
///
/// # Fields
/// * `contract_size` - Multiplier that determines how many of the underlying asset the contract represents.
/// * `settlement_asset` - Asset used for settlement.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PerpetualContract<AssetKey> {
    pub contract_size: Decimal,
    pub settlement_asset: AssetKey,
}
