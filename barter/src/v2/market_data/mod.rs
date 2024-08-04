use crate::v2::engine::error::EngineError;
use barter_data::{event::MarketEvent, instrument::InstrumentId};
use futures::Stream;

/// Todo:
pub async fn init() -> Result<impl Stream<Item = MarketEvent<InstrumentId>>, EngineError> {
    Ok(futures::stream::iter([]))
}
