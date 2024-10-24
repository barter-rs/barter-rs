use futures::Stream;
use barter_instrument::Keyed;
use crate::v2::engine::error::EngineError;
use crate::v2::execution::{AccountEvent, AccountEventKind, ExecutionRequest};
use crate::v2::instrument::Instrument;

pub async fn init<InstrumentKey, AssetKey>(
    _execution_rx: impl Stream<Item = ExecutionRequest<InstrumentKey>>,
    _instruments: &[Keyed<InstrumentKey, Instrument<AssetKey>>],
) -> Result<impl Stream<Item = AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>, EngineError>
{
    Ok(futures::stream::iter([]))
}
