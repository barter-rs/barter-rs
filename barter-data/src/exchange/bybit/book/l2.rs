use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{error::DataError, exchange::bybit::message::BybitPayload};

use super::BybitLevel;

/// Terse type alias for an [`BybitOrderBook`](BybitOrderBook) orderbook WebSocket message.
pub type BybitOrderBook = BybitPayload<BybitOrderBookInner>;

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct BybitOrderBookInner {
    #[serde(rename = "b")]
    pub bids: Vec<BybitLevel>,

    #[serde(rename = "a")]
    pub asks: Vec<BybitLevel>,

    #[serde(rename = "u")]
    pub update_id: u64,

    #[serde(rename = "seq")]
    pub sequence: u64,
}

/// Bybit order book updater
#[derive(Debug, Clone)]
pub struct BybitOrderBookUpdater {
    /// We are using sequence so that we can check which updates were generated
    /// before the HTTP snapshot
    last_sequence: u64,
    /// We are using update_id so that we can check if we got all the updates in
    /// the correct sequence. None means that we still didn't process any updates
    last_update_id: Option<u64>,
}

impl BybitOrderBookUpdater {
    pub fn new(last_sequence: u64) -> Self {
        Self {
            last_sequence,
            last_update_id: None,
        }
    }

    pub fn validate_next_update(&self, update: &BybitOrderBook) -> Result<(), DataError> {
        // This happens when we are processing the first update
        let Some(last_update_id) = self.last_update_id else {
            return Ok(());
        };

        // Check if new update id is correct
        if update.data.update_id != last_update_id + 1 {
            return Err(DataError::InvalidSequence {
                prev_last_update_id: last_update_id,
                first_update_id: update.data.update_id,
            });
        }

        Ok(())
    }
}
