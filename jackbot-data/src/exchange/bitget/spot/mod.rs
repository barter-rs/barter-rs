//! Spot market types and operations for Bitget exchange
pub mod l2;

use jackbot_instrument::exchange::ExchangeId;

/// Bitget Spot Exchange ID
#[derive(Debug, Clone, Copy)]
pub struct BitgetSpot;

impl BitgetSpot {
    /// The exchange ID for Bitget Spot
    pub const ID: ExchangeId = ExchangeId::BitgetSpot;
}
