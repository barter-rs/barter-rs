/// Top-level trading system architecture for composing trading engines with execution components.
///
/// This module provides an architecture for building and running trading systems composed of the
/// `Engine` processor core and various execution components. The system framework abstracts away the
/// low-level concurrency and communication mechanisms, allowing users to focus on implementing
/// trading strategies.
use crate::{
    engine::{
        Processor,
        audit::{AuditTick, Auditor, context::EngineContext},
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
    snapshot::SnapUpdates,
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
{
    /// Task handle for the running `Engine`.
    pub engine: JoinHandle<(Engine, Engine::Audit)>,

    /// Handles to auxiliary system components (execution components, event forwarding, etc.).
    pub handles: SystemAuxillaryHandles,

    /// Transmitter for sending events to the `Engine`.
    pub feed_tx: UnboundedTx<Event>,

    /// Optional audit snapshot with updates (present when audit sending is enabled).
    pub audit:
        Option<SnapUpdates<AuditTick<Engine::Snapshot>, UnboundedRx<AuditTick<Engine::Audit>>>>,
}

impl<Engine, Event> System<Engine, Event>
where
    Engine: Processor<Event> + Auditor<Engine::Audit, Context = EngineContext>,
    Event: Debug + Clone + Send,
{
    /// Shutdown the `System` gracefully.
    pub async fn shutdown(mut self) -> Result<(Engine, Engine::Audit), JoinError>
    where
        Event: From<Shutdown>,
    {
        self.send(Shutdown);

        let (engine, shutdown_audit) = self.engine.await?;

        self.handles.shutdown().await?;

        Ok((engine, shutdown_audit))
    }

    /// Shutdown the `System` ungracefully.
    pub async fn abort(self) -> Result<(Engine, Engine::Audit), JoinError>
    where
        Event: From<Shutdown>,
    {
        self.send(Shutdown);

        let (engine, shutdown_audit) = self.engine.await?;

        self.handles.abort();

        Ok((engine, shutdown_audit))
    }

    /// Shutdown a backtesting `System` gracefully after the `Stream` of `MarketStreamEvent`s has
    /// ended.
    ///
    /// **Note that for live & paper-trading this market stream will never end, so use
    /// System::shutdown() for that use case**.
    pub async fn shutdown_after_backtest(self) -> Result<(Engine, Engine::Audit), JoinError>
    where
        Event: From<Shutdown>,
    {
        let Self {
            engine,
            handles:
                SystemAuxillaryHandles {
                    mut execution,
                    market_to_engine,
                    account_to_engine,
                },
            feed_tx,
            audit: _,
        } = self;

        // Wait for MarketStream to finish forwarding to Engine before initiating Shutdown
        market_to_engine.await?;

        feed_tx
            .send(Shutdown)
            .expect("Engine cannot drop Feed receiver");
        drop(feed_tx);

        let (engine, shutdown_audit) = engine.await?;

        account_to_engine.abort();
        execution.shutdown().await?;

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

    /// Take ownership of the audit snapshot with updates if present.
    ///
    /// Note that by this will not be present if the `System` was built in
    /// [`AuditMode::Disabled`](builder::AuditMode) (default).
    pub fn take_audit(
        &mut self,
    ) -> Option<SnapUpdates<AuditTick<Engine::Snapshot>, UnboundedRx<AuditTick<Engine::Audit>>>>
    {
        self.audit.take()
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
