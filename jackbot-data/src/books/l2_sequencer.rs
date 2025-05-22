use crate::error::DataError;

/// Trait for L2 order book sequencing logic.
pub trait L2Sequencer<Update>: std::fmt::Debug + Send + Sync {
    /// Create a new sequencer from the initial snapshot sequence.
    fn new(last_update_id: u64) -> Self
    where
        Self: Sized;
    /// Validate and process an incoming update. Returns Some(valid_update) if the update should be applied, None if it should be dropped, or Err if there is a sequencing error.
    fn validate_sequence(&mut self, update: Update) -> Result<Option<Update>, DataError>;
    /// Returns true if this is the first update after the snapshot.
    fn is_first_update(&self) -> bool;
}

// Example implementation for Binance Spot
#[derive(Debug, Clone)]
pub struct BinanceSpotOrderBookL2Sequencer {
    pub updates_processed: u64,
    pub last_update_id: u64,
    pub prev_last_update_id: u64,
}

impl BinanceSpotOrderBookL2Sequencer {
    pub fn validate_first_update<Update: HasUpdateIds>(
        &self,
        update: &Update,
    ) -> Result<(), DataError> {
        // U <= lastUpdateId+1 AND u >= lastUpdateId+1
        if update.first_update_id() <= self.last_update_id + 1
            && update.last_update_id() >= self.last_update_id + 1
        {
            Ok(())
        } else {
            Err(DataError::InvalidSequence {
                prev_last_update_id: self.last_update_id,
                first_update_id: update.first_update_id(),
            })
        }
    }
    pub fn validate_next_update<Update: HasUpdateIds>(
        &self,
        update: &Update,
    ) -> Result<(), DataError> {
        // U == prev_last_update_id+1
        if update.first_update_id() == self.prev_last_update_id + 1 {
            Ok(())
        } else {
            Err(DataError::InvalidSequence {
                prev_last_update_id: self.prev_last_update_id,
                first_update_id: update.first_update_id(),
            })
        }
    }
}

impl<Update: HasUpdateIds> L2Sequencer<Update> for BinanceSpotOrderBookL2Sequencer {
    fn new(last_update_id: u64) -> Self {
        Self {
            updates_processed: 0,
            last_update_id,
            prev_last_update_id: last_update_id,
        }
    }
    fn validate_sequence(&mut self, update: Update) -> Result<Option<Update>, DataError> {
        if self.updates_processed == 0 {
            self.validate_first_update(&update)?;
        } else {
            self.validate_next_update(&update)?;
        }
        self.prev_last_update_id = self.last_update_id;
        self.last_update_id = update.last_update_id();
        self.updates_processed += 1;
        Ok(Some(update))
    }
    fn is_first_update(&self) -> bool {
        self.updates_processed == 0
    }
}

pub trait HasUpdateIds {
    fn first_update_id(&self) -> u64;
    fn last_update_id(&self) -> u64;
}

// Example: implement HasUpdateIds for BinanceSpotOrderBookL2Update
// (The actual struct is in binance/spot/l2.rs, so this is just a trait definition for now)
