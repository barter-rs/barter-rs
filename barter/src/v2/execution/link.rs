use crate::v2::{
    engine::error::EngineError,
    execution::{AccountEvent, AccountEventKind, ExecutionRequest},
    instrument::Instrument,
};
use futures::Stream;

pub async fn init<AssetKey, InstrumentKey>(
    _execution_rx: impl Stream<Item = ExecutionRequest<InstrumentKey>>,
    instruments: &[Instrument],
) -> Result<impl Stream<Item = AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>, EngineError>
{
    Ok(futures::stream::iter([]))
}
