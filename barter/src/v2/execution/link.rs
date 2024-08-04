use crate::v2::{
    engine::error::EngineError,
    execution::{AccountEvent, AccountEventKind, ExecutionRequest},
    instrument::Instrument,
};
use barter_instrument::Keyed;
use futures::Stream;

pub async fn init<InstrumentKey, AssetKey>(
    _execution_rx: impl Stream<Item = ExecutionRequest<InstrumentKey>>,
    _instruments: &[Keyed<InstrumentKey, Instrument<AssetKey>>],
) -> Result<impl Stream<Item = AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>, EngineError>
{
    Ok(futures::stream::iter([]))
}
