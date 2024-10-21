use crate::v2::{
    engine::error::EngineError,
    execution::{AccountEvent, AccountEventKind, ExecutionRequest},
    instrument::Instrument,
};
use futures::Stream;
use crate::v2::instrument::KeyedInstrument;

pub async fn init<AssetKey, InstrumentKey>(
    _execution_rx: impl Stream<Item = ExecutionRequest<InstrumentKey>>,
    _instruments: &[KeyedInstrument<InstrumentKey, Instrument>],
) -> Result<impl Stream<Item = AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>, EngineError>
{
    Ok(futures::stream::iter([]))
}
