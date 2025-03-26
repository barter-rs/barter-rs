use crate::{engine::error::UnrecoverableEngineError, execution::request::ExecutionRequest};
use barter_instrument::{
    exchange::{ExchangeId, ExchangeIndex},
    index::error::IndexError,
    instrument::InstrumentIndex,
};
use barter_integration::{
    channel::{Tx, UnboundedTx},
    collection::FnvIndexMap,
};
use std::fmt::Debug;

/// Collection of [`ExecutionRequest`] [`Tx`]s for each
/// exchange [`ExecutionManager`](crate::execution::manager::ExecutionManager).
///
/// Facilitates the routing of execution requests in a multi or single exchange trading system.
pub trait ExecutionTxMap<ExchangeKey = ExchangeIndex, InstrumentKey = InstrumentIndex> {
    type ExecutionTx: Tx<Item = ExecutionRequest<ExchangeKey, InstrumentKey>>;

    /// Attempt to find the [`ExecutionRequest`] [`Tx`] for the provided `ExchangeKey`.
    fn find(&self, exchange: &ExchangeKey) -> Result<&Self::ExecutionTx, UnrecoverableEngineError>;

    /// Returns an `Iterator` of all active [`ExecutionRequest`] [`Tx`]s.
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a Self::ExecutionTx>
    where
        Self::ExecutionTx: 'a;
}

/// A map of exchange transmitters that efficiently routes execution requests to exchange-specific
/// transmitter channels.
///
/// `FnvIndexMap` of [`ExecutionRequest`] [`Tx`]s for each exchange.
///
/// Facilitates the routing of execution requests in a multi exchange trading system.
///
/// Note that a transmitter for an exchange is optional. This handles the case where instruments
/// for an exchange may be tracked by the trading system, but not trading on.
///
/// **Without this optional transmitter the [`ExchangeIndex`]s would not be valid.**.
#[derive(Debug)]
pub struct MultiExchangeTxMap<Tx = UnboundedTx<ExecutionRequest>>(
    FnvIndexMap<ExchangeId, Option<Tx>>,
);

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
    Transmitter: Tx<Item = ExecutionRequest> + Debug,
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

    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a Self::ExecutionTx>
    where
        Self::ExecutionTx: 'a,
    {
        self.0.values().filter_map(|tx| tx.as_ref())
    }
}
