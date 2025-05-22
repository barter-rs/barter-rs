//! Futures market types and operations for Bitget exchange
pub mod l2;

use jackbot_instrument::exchange::ExchangeId;

/// Bitget Futures Exchange ID
#[derive(Debug, Clone, Copy)]
pub struct BitgetFutures;

impl BitgetFutures {
    /// The exchange ID for Bitget Futures
    pub const ID: ExchangeId = ExchangeId::BitgetFutures;
}
