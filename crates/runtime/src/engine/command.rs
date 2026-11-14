use crate::engine::state::instrument::filter::InstrumentFilter;
use serde::{Deserialize, Serialize};
use trading_bot_execution::order::request::{OrderRequestCancel, OrderRequestOpen};
use trading_bot_instrument::{
    asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex,
};
use trading_bot_integration::collection::one_or_many::OneOrMany;

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
