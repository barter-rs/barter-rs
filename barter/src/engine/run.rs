use crate::{
    engine::{
        Processor,
        audit::{AuditTick, Auditable, Auditor, EngineAudit, ProcessAudit, context::EngineContext},
        process_with_audit,
    },
    shutdown::SyncShutdown,
};
use barter_integration::{
    FeedEnded, Terminal,
    channel::{ChannelTxDroppable, Tx},
};
use futures::{Stream, StreamExt};
use std::fmt::Debug;
use tracing::info;

/// Synchronous `Engine` runner that processes input `Events`.
///
/// Runs until shutdown, returning an audit detailing the reason for the shutdown
/// (eg/ `Events` `FeedEnded`, `Command::Shutdown`, etc.).
///
/// # Arguments
/// * `Events` - `Iterator` of events for the `Engine` to process.
/// * `Engine` - Event processor that produces audit events as output.
pub fn sync_run<Events, Engine>(feed: &mut Events, engine: &mut Engine) -> Engine::Output
where
    Events: Iterator,
    Events::Item: Debug + Clone,
    Engine:
        Processor<Events::Item> + Auditor<Engine::Output, Context = EngineContext> + SyncShutdown,
    Engine::Output: From<FeedEnded> + Terminal + Debug,
{
    info!(
        feed_mode = "sync",
        audit_mode = "disabled",
        "Engine running"
    );

    // Run Engine process loop until shutdown
    let shutdown_audit = loop {
        let Some(event) = feed.next() else {
            break engine.audit(FeedEnded);
        };

        // Process Event with AuditTick generation
        let audit = process_with_audit(engine, event);

        // Check if AuditTick indicates a shutdown is required
        if audit.event.is_terminal() {
            break audit;
        }
    };

    info!(
        shutdown_audit = ?shutdown_audit.event,
        context = ?shutdown_audit.context,
        "Engine shutting down"
    );

    let _ = engine.shutdown();

    shutdown_audit.event
}

pub fn sync_run_new<Inputs, Engine, Output>(
    feed: &mut Inputs,
    engine: &mut Engine,
) -> EngineAudit<Inputs::Item, Output>
where
    Inputs: Iterator,
    Inputs::Item: Debug + Clone,
    Engine: for<'a> Processor<&'a Inputs::Item, Output = Output> + Auditable + SyncShutdown,
    Engine::Context: Debug,
    Output: Terminal + Debug,
{
    info!(
        feed_mode = "sync",
        audit_mode = "disabled",
        "Engine running"
    );

    // Run Engine process loop until shutdown
    let shutdown_audit = loop {
        let Some(input) = feed.next() else {
            break AuditTick {
                event: EngineAudit::FeedEnded,
                context: engine.context(),
            };
        };

        let output = engine.process(&input);

        // Check if Engine::Output indicates a shutdown is required
        if output.is_terminal() {
            break AuditTick {
                event: EngineAudit::Process(ProcessAudit { input, output }),
                context: engine.context(),
            };
        }
    };

    info!(
        shutdown_audit = ?shutdown_audit.event,
        context = ?shutdown_audit.context,
        "Engine shutting down"
    );

    let _ = engine.shutdown();

    shutdown_audit.event
}

pub fn sync_run_new_with_audit<Inputs, Engine, Output, FnAudit>(
    feed: &mut Inputs,
    engine: &mut Engine,
    mut fn_audit: FnAudit,
) -> EngineAudit<Inputs::Item, Output>
where
    Inputs: Iterator,
    Inputs::Item: Debug + Clone,
    Engine: for<'a> Processor<&'a Inputs::Item, Output = Output> + Auditable + SyncShutdown,
    Engine::Context: Debug + Clone,
    Output: Terminal + Debug + Clone,
    FnAudit: FnMut(AuditTick<EngineAudit<Inputs::Item, Output>, Engine::Context>),
{
    info!(
        feed_mode = "sync",
        audit_mode = "disabled",
        "Engine running"
    );

    // Run Engine process loop until shutdown
    let shutdown_audit = loop {
        let Some(input) = feed.next() else {
            break AuditTick {
                event: EngineAudit::FeedEnded,
                context: engine.context(),
            };
        };

        let output = engine.process(&input);

        let is_terminal = output.is_terminal();

        let audit = AuditTick {
            event: EngineAudit::Process(ProcessAudit { input, output }),
            context: engine.context(),
        };

        // Check if Engine::Output indicates a shutdown is required
        if is_terminal {
            break audit;
        }

        // Send AuditTick via FnAudit
        (fn_audit)(audit)
    };

    // Send Shutdown AuditTick via FnAudit
    (fn_audit)(shutdown_audit.clone());

    info!(
        shutdown_audit = ?shutdown_audit.event,
        context = ?shutdown_audit.context,
        "Engine shutting down"
    );

    let _ = engine.shutdown();

    shutdown_audit.event
}

/// Synchronous `Engine` runner that processes input `Events` and forwards audits to the provided
/// `AuditTx`.
///
/// Runs until shutdown, returning an audit detailing the reason for the shutdown
/// (eg/ `Events` `FeedEnded`, `Command::Shutdown`, etc.).
///
/// # Arguments
/// * `Events` - `Iterator` of events for the `Engine` to process.
/// * `Engine` - Event processor that produces audit events as output.
/// * `AuditTx` - Channel for sending produced audit events.
pub fn sync_run_with_audit<Events, Engine, AuditTx>(
    feed: &mut Events,
    engine: &mut Engine,
    audit_tx: &mut ChannelTxDroppable<AuditTx>,
) -> Engine::Output
where
    Events: Iterator,
    Events::Item: Debug + Clone,
    Engine:
        Processor<Events::Item> + Auditor<Engine::Output, Context = EngineContext> + SyncShutdown,
    Engine::Output: From<FeedEnded> + Terminal + Debug + Clone,
    AuditTx: Tx<Item = AuditTick<Engine::Output, EngineContext>>,
{
    info!(feed_mode = "sync", audit_mode = "enabled", "Engine running");

    // Run Engine process loop until shutdown
    let shutdown_audit = loop {
        let Some(event) = feed.next() else {
            break engine.audit(FeedEnded);
        };

        // Process Event with AuditTick generation
        let audit = process_with_audit(engine, event);

        // Check if AuditTick indicates shutdown is required
        if audit.event.is_terminal() {
            break audit;
        }

        // Send AuditTick to AuditManager
        audit_tx.send(audit);
    };

    // Send Shutdown audit
    audit_tx.send(shutdown_audit.clone());

    info!(
        shutdown_audit = ?shutdown_audit.event,
        context = ?shutdown_audit.context,
        "Engine shutting down"
    );

    let _ = engine.shutdown();

    shutdown_audit.event
}

pub async fn async_run_new<Inputs, Engine, Output>(
    feed: &mut Inputs,
    engine: &mut Engine,
) -> EngineAudit<Inputs::Item, Output>
where
    Inputs: Stream + Unpin,
    Inputs::Item: Debug + Clone,
    Engine: for<'a> Processor<&'a Inputs::Item, Output = Output> + Auditable + SyncShutdown,
    Engine::Context: Debug,
    Output: Terminal + Debug,
{
    info!(
        feed_mode = "async",
        audit_mode = "disabled",
        "Engine running"
    );

    // Run Engine process loop until shutdown
    let shutdown_audit = loop {
        let Some(input) = feed.next().await else {
            break AuditTick {
                event: EngineAudit::FeedEnded,
                context: engine.context(),
            };
        };

        let output = engine.process(&input);

        // Check if Engine::Output indicates a shutdown is required
        if output.is_terminal() {
            break AuditTick {
                event: EngineAudit::Process(ProcessAudit { input, output }),
                context: engine.context(),
            };
        }
    };

    info!(
        shutdown_audit = ?shutdown_audit.event,
        context = ?shutdown_audit.context,
        "Engine shutting down"
    );

    let _ = engine.shutdown();

    shutdown_audit.event
}

pub async fn async_run_new_with_audit<Inputs, Engine, Output, FnAudit>(
    feed: &mut Inputs,
    engine: &mut Engine,
    mut fn_audit: FnAudit,
) -> EngineAudit<Inputs::Item, Output>
where
    Inputs: Stream + Unpin,
    Inputs::Item: Debug + Clone,
    Engine: for<'a> Processor<&'a Inputs::Item, Output = Output> + Auditable + SyncShutdown,
    Engine::Context: Debug + Clone,
    Output: Terminal + Debug + Clone,
    FnAudit: FnMut(AuditTick<EngineAudit<Inputs::Item, Output>, Engine::Context>),
{
    info!(
        feed_mode = "async",
        audit_mode = "enabled",
        "Engine running"
    );

    // Run Engine process loop until shutdown
    let shutdown_audit = loop {
        let Some(input) = feed.next().await else {
            break AuditTick {
                event: EngineAudit::FeedEnded,
                context: engine.context(),
            };
        };

        let output = engine.process(&input);

        let is_terminal = output.is_terminal();

        let audit = AuditTick {
            event: EngineAudit::Process(ProcessAudit { input, output }),
            context: engine.context(),
        };

        // Check if Engine::Output indicates a shutdown is required
        if is_terminal {
            break audit;
        }

        // Send AuditTick via FnAudit
        (fn_audit)(audit);
    };

    // Send Shutdown AuditTick via FnAudit
    (fn_audit)(shutdown_audit.clone());

    info!(
        shutdown_audit = ?shutdown_audit.event,
        context = ?shutdown_audit.context,
        "Engine shutting down"
    );

    let _ = engine.shutdown();

    shutdown_audit.event
}

/// Asynchronous `Engine` runner that processes input `Events`.
///
/// Runs until shutdown, returning an audit detailing the reason for the shutdown
/// (eg/ `Events` `FeedEnded`, `Command::Shutdown`, etc.).
///
/// # Arguments
/// * `Events` - `Stream` of events for the `Engine` to process.
/// * `Engine` - Event processor that produces audit events as output.
/// * `AuditTx` - Channel for sending produced audit events.
pub async fn async_run<Events, Engine>(feed: &mut Events, engine: &mut Engine) -> Engine::Output
where
    Events: Stream + Unpin,
    Events::Item: Debug + Clone,
    Engine:
        Processor<Events::Item> + Auditor<Engine::Output, Context = EngineContext> + SyncShutdown,
    Engine::Output: From<FeedEnded> + Terminal + Debug,
{
    info!(
        feed_mode = "async",
        audit_mode = "disabled",
        "Engine running"
    );

    // Run Engine process loop until shutdown
    let shutdown_audit = loop {
        let Some(event) = feed.next().await else {
            break engine.audit(FeedEnded);
        };

        // Process Event with AuditTick generation
        let audit = process_with_audit(engine, event);

        // Check if AuditTick indicates shutdown is required
        if audit.event.is_terminal() {
            break audit;
        }
    };

    info!(
        shutdown_audit = ?shutdown_audit.event,
        context = ?shutdown_audit.context,
        "Engine shutting down"
    );

    let _ = engine.shutdown();

    shutdown_audit.event
}

/// Asynchronous `Engine` runner that processes input `Events` and forwards audits to the provided
/// `AuditTx`.
///
/// Runs until shutdown, returning an audit detailing the reason for the shutdown
/// (eg/ `Events` `FeedEnded`, `Command::Shutdown`, etc.).
///
/// # Arguments
/// * `Events` - `Stream` of events for the `Engine` to process.
/// * `Engine` - Event processor that produces audit events as output.
/// * `AuditTx` - Channel for sending produced audit events.
pub async fn async_run_with_audit<Events, Engine, AuditTx>(
    feed: &mut Events,
    engine: &mut Engine,
    audit_tx: &mut ChannelTxDroppable<AuditTx>,
) -> Engine::Output
where
    Events: Stream + Unpin,
    Events::Item: Debug + Clone,
    Engine:
        Processor<Events::Item> + Auditor<Engine::Output, Context = EngineContext> + SyncShutdown,
    Engine::Output: From<FeedEnded> + Terminal + Debug + Clone,
    AuditTx: Tx<Item = AuditTick<Engine::Output, EngineContext>>,
{
    info!(
        feed_mode = "async",
        audit_mode = "enabled",
        "Engine running"
    );

    // Run Engine process loop until shutdown
    let shutdown_audit = loop {
        let Some(event) = feed.next().await else {
            break engine.audit(FeedEnded);
        };

        // Process Event with AuditTick generation
        let audit = process_with_audit(engine, event);

        // Check if AuditTick indicates shutdown is required
        if audit.event.is_terminal() {
            break audit;
        }

        // Send AuditTick to AuditManager
        audit_tx.send(audit);
    };

    // Send Shutdown audit
    audit_tx.send(shutdown_audit.clone());

    info!(
        shutdown_audit = ?shutdown_audit.event,
        context = ?shutdown_audit.context,
        "Engine shutting down"
    );

    let _ = engine.shutdown();

    shutdown_audit.event
}
