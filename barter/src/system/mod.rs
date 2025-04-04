/// Top-level trading system architecture for composing trading engines with execution components.
///
/// This module provides an architecture for building and running trading systems composed of the
/// `Engine` processor core and various execution components. The system framework abstracts away the
/// low-level concurrency and communication mechanisms, allowing users to focus on implementing
/// trading strategies.
use crate::{
    engine::{
        Processor,
        audit::{AuditTick, Auditor, context::EngineContext, shutdown::ShutdownAudit},
        command::Command,
        state::{instrument::filter::InstrumentFilter, trading::TradingState},
    },
    execution::builder::ExecutionHandles,
    shutdown::{AsyncShutdown, Shutdown},
};
use barter_execution::order::request::{OrderRequestCancel, OrderRequestOpen};
use barter_integration::{
    channel::{Tx, UnboundedRx, UnboundedTx},
    collection::one_or_many::OneOrMany,
};
use std::fmt::Debug;
use tokio::task::{JoinError, JoinHandle};

/// Provides a `SystemBuilder` for constructing a Barter trading system, and associated types.
pub mod builder;

/// Provides a convenient `SystemConfig` used for defining a Barter trading system.
pub mod config;

/// Initialised and running Barter trading system.
///
/// Contains handles for the `Engine` and all auxillary system tasks.
///
/// It provides methods for interacting with the system, such as sending `Engine` [`Command`]s,
/// managing [`TradingState`], and shutting down gracefully.
#[allow(missing_debug_implementations)]
pub struct System<Engine, Event>
where
    Engine: Processor<Event> + Auditor<Engine::Audit, Context = EngineContext>,
    Engine::Audit: From<Engine::Snapshot>,
{
    /// Task handle for the running `Engine`.
    engine: JoinHandle<(Engine, ShutdownAudit<Event, Engine::Output>)>,

    /// Handles to auxiliary system components (execution components, event forwarding, etc.).
    pub handles: SystemAuxillaryHandles,

    /// Transmitter for sending events to the `Engine`.
    pub feed_tx: UnboundedTx<Event>,

    /// Optional receiver for engine audit events (present when audit sending is enabled).
    pub audit_rx: Option<UnboundedRx<AuditTick<Engine::Audit, EngineContext>>>,
}

impl<Engine, Event> System<Engine, Event>
where
    Engine: Processor<Event> + Auditor<Engine::Audit, Context = EngineContext>,
    Engine::Audit: From<Engine::Snapshot>,
    Event: Debug + Clone + Send,
{
    /// Shutdown the `System` gracefully.
    pub async fn shutdown(
        mut self,
    ) -> Result<(Engine, ShutdownAudit<Event, Engine::Output>), JoinError>
    where
        Event: From<Shutdown>,
    {
        self.send(Shutdown);

        let (engine, shutdown_audit) = self.engine.await?;

        self.handles.shutdown().await?;

        Ok((engine, shutdown_audit))
    }

    /// Shutdown the `System` ungracefully.
    pub async fn abort(self) -> Result<(Engine, ShutdownAudit<Event, Engine::Output>), JoinError>
    where
        Event: From<Shutdown>,
    {
        self.send(Shutdown);

        let (engine, shutdown_audit) = self.engine.await?;

        self.handles.abort();

        Ok((engine, shutdown_audit))
    }

    /// Send [`OrderRequestCancel`]s to the `Engine` for execution.
    pub fn send_cancel_requests(&self, requests: OneOrMany<OrderRequestCancel>)
    where
        Event: From<Command>,
    {
        self.send(Command::SendCancelRequests(requests))
    }

    /// Send [`OrderRequestOpen`]s to the `Engine` for execution.
    pub fn send_open_requests(&self, requests: OneOrMany<OrderRequestOpen>)
    where
        Event: From<Command>,
    {
        self.send(Command::SendOpenRequests(requests))
    }

    /// Instruct the `Engine` to close open positions.
    ///
    /// Use the `InstrumentFilter` to configure which positions are closed.
    pub fn close_positions(&self, filter: InstrumentFilter)
    where
        Event: From<Command>,
    {
        self.send(Command::ClosePositions(filter))
    }

    /// Instruct the `Engine` to cancel open orders.
    ///
    /// Use the `InstrumentFilter` to configure which orders are cancelled.
    pub fn cancel_orders(&self, filter: InstrumentFilter)
    where
        Event: From<Command>,
    {
        self.send(Command::CancelOrders(filter))
    }

    /// Update the algorithmic `TradingState` of the `Engine`.
    pub fn trading_state(&self, trading_state: TradingState)
    where
        Event: From<TradingState>,
    {
        self.send(trading_state)
    }

    /// Take ownership of the audit channel receiver if present.
    ///
    /// Note that by this will not be present if the `System` was built in
    /// [`AuditMode::Disabled`] (default).
    pub fn take_audit_rx(
        &mut self,
    ) -> Option<UnboundedRx<AuditTick<Engine::Audit, EngineContext>>> {
        self.audit_rx.take()
    }

    /// Send an `Event` to the `Engine`.
    fn send<T>(&self, event: T)
    where
        T: Into<Event>,
    {
        self.feed_tx
            .send(event)
            .expect("Engine cannot drop Feed receiver")
    }
}

/// Collection of task handles for auxiliary system components that support the `Engine`.
///
/// Used by the [`System`] to shut down auxillary components.
#[allow(missing_debug_implementations)]
pub struct SystemAuxillaryHandles {
    /// Handles for running execution components.
    pub execution: ExecutionHandles,

    /// Task that forwards market events to the engine.
    pub market_to_engine: JoinHandle<()>,

    /// Task that forwards account events to the engine.
    pub account_to_engine: JoinHandle<()>,
}

impl AsyncShutdown for SystemAuxillaryHandles {
    type Result = Result<(), JoinError>;

    async fn shutdown(&mut self) -> Self::Result {
        // Event -> Engine tasks do not need graceful shutdown, so abort
        self.market_to_engine.abort();
        self.account_to_engine.abort();

        // Await execution components shutdowns concurrently
        self.execution.shutdown().await
    }
}

impl SystemAuxillaryHandles {
    pub fn abort(self) {
        self.execution
            .into_iter()
            .chain(std::iter::once(self.market_to_engine))
            .chain(std::iter::once(self.account_to_engine))
            .for_each(|handle| handle.abort());
    }
}
