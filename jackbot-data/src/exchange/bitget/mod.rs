//! Exchange module for Bitget. Implements all required traits and re-exports submodules.
pub mod channel;
// pub mod l1;
// pub mod l2;
pub mod book;
pub mod futures;
pub mod liquidation;
pub mod market;
pub mod spot;
pub mod subscription;
pub mod trade;

/// Rate limiting utilities for Bitget.
pub mod rate_limit;
