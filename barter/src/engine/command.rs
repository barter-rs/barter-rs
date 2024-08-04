use crate::engine::state::instrument::filter::InstrumentFilter;
use barter_execution::order::{Order, RequestCancel, RequestOpen};
use barter_instrument::{asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex};
use barter_integration::collection::one_or_many::OneOrMany;
use serde::{Deserialize, Serialize};

/// Trading related commands for the [`Engine`](super::Engine) to action, sent from an
/// external process.
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub enum Command<
    ExchangeKey = ExchangeIndex,
    AssetKey = AssetIndex,
    InstrumentKey = InstrumentIndex,
> {
    SendCancelRequests(OneOrMany<Order<ExchangeKey, InstrumentKey, RequestCancel>>),
    SendOpenRequests(OneOrMany<Order<ExchangeKey, InstrumentKey, RequestOpen>>),
    ClosePositions(InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>),
    CancelOrders(InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>),
}
