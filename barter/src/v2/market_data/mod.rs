use crate::v2::engine::error::EngineError;
use barter_data::event::MarketEvent;
use futures::Stream;

/// Todo:
pub async fn init<InstrumentKey>(
) -> Result<impl Stream<Item = MarketEvent<InstrumentKey>>, EngineError> {
    Ok(futures::stream::iter([]))
}
