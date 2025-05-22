use crate::subscription::liquidation::Liquidations;
use jackbot_integration::subscription::SubscriptionId;

/// OKX does not support a public liquidations channel as of 2024-06.
/// This module is a stub for feature parity and will not emit any events.
pub struct OkxLiquidation;

impl OkxLiquidation {
    pub fn from_message(_msg: &str) -> Option<()> {
        // No public liquidations channel on OKX
        None
    }
}

impl crate::Identifier<Option<SubscriptionId>> for OkxLiquidation {
    fn id(&self) -> Option<SubscriptionId> {
        None
    }
}

// TODO: Implement normalization from raw OKX messages to Liquidation
