use crate::v2::{
    engine::error::EngineError,
    order::{Order, RequestCancel, RequestOpen},
};
use barter_integration::{collection::none_one_or_many::NoneOneOrMany, Unrecoverable};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SentRequestsAudit<ExchangeKey, InstrumentKey> {
    pub cancels: NoneOneOrMany<Order<ExchangeKey, InstrumentKey, RequestCancel>>,
    pub opens: NoneOneOrMany<Order<ExchangeKey, InstrumentKey, RequestOpen>>,
    pub failed_cancels: NoneOneOrMany<(
        Order<ExchangeKey, InstrumentKey, RequestCancel>,
        EngineError,
    )>,
    pub failed_opens: NoneOneOrMany<(Order<ExchangeKey, InstrumentKey, RequestOpen>, EngineError)>,
}

impl<ExchangeKey, InstrumentKey> Unrecoverable for SentRequestsAudit<ExchangeKey, InstrumentKey> {
    fn is_unrecoverable(&self) -> bool {
        self.failed_cancels
            .iter()
            .any(|(_, error)| error.is_unrecoverable())
            || self
                .failed_opens
                .iter()
                .any(|(_, error)| error.is_unrecoverable())
    }
}

impl<ExchangeKey, InstrumentKey> Default for SentRequestsAudit<ExchangeKey, InstrumentKey> {
    fn default() -> Self {
        Self {
            cancels: NoneOneOrMany::None,
            opens: NoneOneOrMany::None,
            failed_cancels: NoneOneOrMany::None,
            failed_opens: NoneOneOrMany::None,
        }
    }
}
