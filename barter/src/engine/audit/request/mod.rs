use crate::engine::{
    audit::request::sent::SentRequestsAudit,
    error::{EngineError, UnrecoverableEngineError},
};
use barter_integration::{collection::one_or_many::OneOrMany, Unrecoverable};
use derive_more::Constructor;
use risk_refused::RiskRefusedRequestsAudit;
use serde::{Deserialize, Serialize};

pub mod risk_refused;
pub mod sent;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct ExecutionRequestAudit<ExchangeKey, InstrumentKey> {
    pub sent: SentRequestsAudit<ExchangeKey, InstrumentKey>,
    pub refused: RiskRefusedRequestsAudit<ExchangeKey, InstrumentKey>,
}

impl<ExchangeKey, InstrumentKey> ExecutionRequestAudit<ExchangeKey, InstrumentKey> {
    pub fn unrecoverable_errors(&self) -> Option<OneOrMany<UnrecoverableEngineError>> {
        if !self.is_unrecoverable() {
            return None;
        }

        Some(
            self.sent
                .failed_cancels
                .iter()
                .filter_map(|(_order, error)| match error {
                    EngineError::Unrecoverable(error) => Some(error.clone()),
                    _ => None,
                })
                .chain(
                    self.sent
                        .failed_opens
                        .iter()
                        .filter_map(|(_order, error)| match error {
                            EngineError::Unrecoverable(error) => Some(error.clone()),
                            _ => None,
                        }),
                )
                .collect(),
        )
    }
}

impl<ExchangeKey, InstrumentKey> Unrecoverable
    for ExecutionRequestAudit<ExchangeKey, InstrumentKey>
{
    fn is_unrecoverable(&self) -> bool {
        self.sent.is_unrecoverable()
    }
}

impl<ExchangeKey, InstrumentKey> From<SentRequestsAudit<ExchangeKey, InstrumentKey>>
    for ExecutionRequestAudit<ExchangeKey, InstrumentKey>
{
    fn from(value: SentRequestsAudit<ExchangeKey, InstrumentKey>) -> Self {
        Self::new(value, RiskRefusedRequestsAudit::default())
    }
}
