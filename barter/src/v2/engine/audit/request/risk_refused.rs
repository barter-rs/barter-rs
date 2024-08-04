use crate::v2::{
    order::{Order, RequestCancel, RequestOpen},
    risk::RiskRefused,
};
use barter_integration::collection::none_one_or_many::NoneOneOrMany;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct RiskRefusedRequestsAudit<ExchangeKey, InstrumentKey> {
    pub refused_cancels:
        NoneOneOrMany<RiskRefused<Order<ExchangeKey, InstrumentKey, RequestCancel>>>,
    pub refused_opens: NoneOneOrMany<RiskRefused<Order<ExchangeKey, InstrumentKey, RequestOpen>>>,
}

impl<ExchangeKey, InstrumentKey> Default for RiskRefusedRequestsAudit<ExchangeKey, InstrumentKey> {
    fn default() -> Self {
        Self::new(NoneOneOrMany::None, NoneOneOrMany::None)
    }
}
