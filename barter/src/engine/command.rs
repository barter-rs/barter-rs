use crate::engine::state::instrument::filter::InstrumentFilter;
use barter_execution::order::request::{OrderRequestCancel, OrderRequestOpen};
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
    SendCancelRequests(OneOrMany<OrderRequestCancel<ExchangeKey, InstrumentKey>>),
    SendOpenRequests(OneOrMany<OrderRequestOpen<ExchangeKey, InstrumentKey>>),
    ClosePositions(InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>),
    CancelOrders(InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>),
}
