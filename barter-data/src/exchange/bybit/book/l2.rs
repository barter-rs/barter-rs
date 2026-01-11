use crate::{
    error::DataError,
    exchange::bybit::message::BybitPayloadKind,
    subscription::book::OrderBookEvent,
    transformer::sequenced::OrderBookL2Sequencer,
};
use barter_integration::subscription::SubscriptionId;
use super::BybitOrderBookMessage;
use tracing::debug;

#[derive(Debug)]
pub struct BybitOrderBookL2Sequencer {
    last_update_id: Option<u64>,
}

impl OrderBookL2Sequencer for BybitOrderBookL2Sequencer {
    type Update = BybitOrderBookMessage;

    fn new(_: Option<&OrderBookEvent>, _: SubscriptionId) -> Result<Self, DataError> {
        Ok(Self { last_update_id: None })
    }

    fn validate(&mut self, update: Self::Update) -> Result<Option<Self::Update>, DataError> {
        if matches!(update.kind, BybitPayloadKind::Snapshot) {
            self.last_update_id = Some(update.data.update_id);
            return Ok(Some(update));
        }

        if let Some(last_update_id) = self.last_update_id {
            if update.data.update_id != last_update_id + 1 {
                return Err(DataError::InvalidSequence {
                    prev_last_update_id: last_update_id,
                    first_update_id: update.data.update_id,
                });
            }
            self.last_update_id = Some(update.data.update_id);
            Ok(Some(update))
        } else {
            debug!("Update message received before initial Snapshot");
            Ok(None)
        }
    }
}
