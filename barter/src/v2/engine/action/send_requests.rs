use crate::v2::{
    engine::{
        error::{EngineError, RecoverableEngineError, UnrecoverableEngineError},
        execution_tx::ExecutionTxMap,
        Engine,
    },
    execution::ExecutionRequest,
    order::Order,
};
use barter_integration::{
    channel::Tx,
    collection::{none_one_or_many::NoneOneOrMany, one_or_many::OneOrMany},
    Unrecoverable,
};
use derive_more::Constructor;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::error;

impl<State, ExecutionTxs, Strategy, Risk> Engine<State, ExecutionTxs, Strategy, Risk> {
    pub fn send_requests<ExchangeKey, InstrumentKey, Kind>(
        &self,
        requests: impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, Kind>>,
    ) -> SendRequestsOutput<ExchangeKey, InstrumentKey, Kind>
    where
        ExecutionTxs: ExecutionTxMap<ExchangeKey, InstrumentKey>,
        ExchangeKey: Debug + Clone,
        InstrumentKey: Debug + Clone,
        Kind: Debug + Clone,
        ExecutionRequest<ExchangeKey, InstrumentKey>: From<Order<ExchangeKey, InstrumentKey, Kind>>,
    {
        // Send order requests
        let (sent, errors): (Vec<_>, Vec<_>) = requests
            .into_iter()
            .map(|request| {
                self.send_request(&request)
                    .map_err(|error| (request.clone(), error))
                    .map(|_| request)
            })
            .partition_result();

        SendRequestsOutput::new(NoneOneOrMany::from(sent), NoneOneOrMany::from(errors))
    }

    pub fn send_request<ExchangeKey, InstrumentKey, Kind>(
        &self,
        request: &Order<ExchangeKey, InstrumentKey, Kind>,
    ) -> Result<(), EngineError>
    where
        ExecutionTxs: ExecutionTxMap<ExchangeKey, InstrumentKey>,
        ExchangeKey: Debug + Clone,
        InstrumentKey: Debug + Clone,
        Kind: Debug + Clone,
        ExecutionRequest<ExchangeKey, InstrumentKey>: From<Order<ExchangeKey, InstrumentKey, Kind>>,
    {
        match self
            .execution_txs
            .find(&request.exchange)?
            .send(ExecutionRequest::from(request.clone()))
        {
            Ok(()) => Ok(()),
            Err(error) if error.is_unrecoverable() => {
                error!(
                    exchange = ?request.exchange,
                    ?request,
                    ?error,
                    "failed to send ExecutionRequest due to terminated channel"
                );
                Err(EngineError::Unrecoverable(
                    UnrecoverableEngineError::ExecutionChannelTerminated(format!(
                        "{:?} execution channel terminated: {:?}",
                        request.exchange, error
                    )),
                ))
            }
            Err(error) => {
                error!(
                    exchange = ?request.exchange,
                    ?request,
                    ?error,
                    "failed to send ExecutionRequest due to unhealthy channel"
                );
                Err(EngineError::Recoverable(
                    RecoverableEngineError::ExecutionChannelUnhealthy(format!(
                        "{:?} execution channel unhealthy: {:?}",
                        request.exchange, error
                    )),
                ))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct SendRequestsOutput<ExchangeKey, InstrumentKey, Kind> {
    pub sent: NoneOneOrMany<Order<ExchangeKey, InstrumentKey, Kind>>,
    pub errors: NoneOneOrMany<(Order<ExchangeKey, InstrumentKey, Kind>, EngineError)>,
}

impl<ExchangeKey, InstrumentKey, Kind> SendRequestsOutput<ExchangeKey, InstrumentKey, Kind> {
    pub fn unrecoverable_errors(&self) -> Option<OneOrMany<UnrecoverableEngineError>> {
        // Send requests output contains no unrecoverable errors
        if !self.is_unrecoverable() {
            return None;
        }

        Some(
            self.errors
                .iter()
                .filter_map(|(order, error)| match error {
                    EngineError::Unrecoverable(error) => Some(error.clone()),
                    _ => None,
                })
                .collect(),
        )
    }
}

impl<ExchangeKey, InstrumentKey, Kind> Unrecoverable
    for SendRequestsOutput<ExchangeKey, InstrumentKey, Kind>
{
    fn is_unrecoverable(&self) -> bool {
        self.errors
            .iter()
            .any(|(_, error)| error.is_unrecoverable())
    }
}
