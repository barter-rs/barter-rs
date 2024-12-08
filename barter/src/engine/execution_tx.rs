use crate::{
    engine::error::UnrecoverableEngineError,
    execution::request::{ExecutionRequest, IndexedExecutionRequest},
    FnvIndexMap,
};
use barter_instrument::{
    exchange::{ExchangeId, ExchangeIndex},
    index::error::IndexError,
    instrument::InstrumentIndex,
};
use barter_integration::channel::Tx;
use std::fmt::Debug;

pub trait ExecutionTxMap<ExchangeKey, InstrumentKey> {
    type ExecutionTx: Tx<Item = ExecutionRequest<ExchangeKey, InstrumentKey>>;
    fn find(&self, exchange: &ExchangeKey) -> Result<&Self::ExecutionTx, UnrecoverableEngineError>;
}

#[derive(Debug)]
pub struct MultiExchangeTxMap<Tx>(FnvIndexMap<ExchangeId, Option<Tx>>);

impl<Tx> FromIterator<(ExchangeId, Option<Tx>)> for MultiExchangeTxMap<Tx> {
    fn from_iter<Iter>(iter: Iter) -> Self
    where
        Iter: IntoIterator<Item = (ExchangeId, Option<Tx>)>,
    {
        MultiExchangeTxMap(FnvIndexMap::from_iter(iter))
    }
}

impl<'a, Tx> IntoIterator for &'a MultiExchangeTxMap<Tx> {
    type Item = (&'a ExchangeId, &'a Option<Tx>);
    type IntoIter = indexmap::map::Iter<'a, ExchangeId, Option<Tx>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, Tx> IntoIterator for &'a mut MultiExchangeTxMap<Tx> {
    type Item = (&'a ExchangeId, &'a mut Option<Tx>);
    type IntoIter = indexmap::map::IterMut<'a, ExchangeId, Option<Tx>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl<Transmitter> ExecutionTxMap<ExchangeIndex, InstrumentIndex> for MultiExchangeTxMap<Transmitter>
where
    Transmitter: Tx<Item = IndexedExecutionRequest> + Debug,
{
    type ExecutionTx = Transmitter;

    fn find(
        &self,
        exchange: &ExchangeIndex,
    ) -> Result<&Self::ExecutionTx, UnrecoverableEngineError> {
        self.0
            .get_index(exchange.index())
            .and_then(|(_exchange, tx)| tx.as_ref())
            .ok_or_else(|| {
                UnrecoverableEngineError::IndexError(IndexError::ExchangeIndex(format!(
                    "failed to find ExecutionTx for ExchangeIndex: {exchange}. Available: {self:?}"
                )))
            })
    }
}
