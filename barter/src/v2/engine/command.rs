use crate::v2::{
    engine::state::instrument::manager::InstrumentFilter,
    order::{Order, RequestCancel, RequestOpen},
};
use barter_integration::collection::one_or_many::OneOrMany;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub enum Command<ExchangeKey, AssetKey, InstrumentKey> {
    SendCancelRequests(OneOrMany<Order<ExchangeKey, InstrumentKey, RequestCancel>>),
    SendOpenRequests(OneOrMany<Order<ExchangeKey, InstrumentKey, RequestOpen>>),
    ClosePositions(InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>),
    CancelOrders(InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>),
}
