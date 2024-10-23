// use crate::v2::{
//     engine::error::EngineError,
//     execution::{AccountEvent, AccountEventKind, ExecutionRequest},
// <<<<<<< Updated upstream
//     instrument::Instrument,
// =======
//     instrument::{Instrument, Keyed},
// >>>>>>> Stashed changes
// };
// use barter_instrument::Keyed;
// use futures::Stream;
//
// pub async fn init<InstrumentKey, AssetKey>(
//     _execution_rx: impl Stream<Item = ExecutionRequest<InstrumentKey>>,
// <<<<<<< Updated upstream
//     _instruments: &[Keyed<InstrumentKey, Instrument<AssetKey>>],
// =======
//     _instruments: &[Keyed<InstrumentKey, Instrument>],
// >>>>>>> Stashed changes
// ) -> Result<impl Stream<Item = AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>, EngineError>
// {
//     Ok(futures::stream::iter([]))
// }
