use crate::engine::{
    Processor,
    audit::{AuditTick, Auditor, context::EngineContext, shutdown::ShutdownAudit},
    process_with_audit,
};
use barter_integration::channel::{ChannelTxDroppable, Tx};
use futures::{Stream, StreamExt};
use std::fmt::Debug;
use tracing::info;

/// Synchronous `Engine` runner that processes input `Events`.
///
/// Runs until shutdown, returning a [`ShutdownAudit`] detailing the reason for the shutdown
/// (eg/ `Events` `FeedEnded`, `Command::Shutdown`, etc.).
///
/// # Arguments
/// * `Events` - `Iterator` of events for the `Engine` to process.
/// * `Engine` - Event processor that produces audit events as output.
pub fn sync_run<Events, Engine>(
    feed: &mut Events,
    engine: &mut Engine,
) -> ShutdownAudit<Events::Item, Engine::Output>
where
    Events: Iterator,
    Events::Item: Debug + Clone,
    Engine: Processor<Events::Item> + Auditor<Engine::Audit, Context = EngineContext>,
    Engine::Audit: From<Engine::Snapshot> + From<ShutdownAudit<Events::Item, Engine::Output>>,
    Engine::Output: Debug + Clone,
    Option<ShutdownAudit<Events::Item, Engine::Output>>: for<'a> From<&'a Engine::Audit>,
{
    info!(
        feed_mode = "sync",
        audit_mode = "disabled",
        "Engine running"
    );

    // Run Engine process loop until shutdown
    let shutdown_audit = loop {
        let Some(event) = feed.next() else {
            break ShutdownAudit::FeedEnded;
        };

        // Process Event with AuditTick generation
        let audit = process_with_audit(engine, event);

        // Check if AuditTick indicates shutdown is required
        let shutdown = Option::<ShutdownAudit<Events::Item, Engine::Output>>::from(&audit.event);

        if let Some(shutdown) = shutdown {
            break shutdown;
        }
    };

    info!(?shutdown_audit, "Engine shutting down");
    shutdown_audit
}

/// Synchronous `Engine` runner that processes input `Events` and forwards audits to the provided
/// `AuditTx`.
///
/// Runs until shutdown, returning a [`ShutdownAudit`] detailing the reason for the shutdown
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
) -> ShutdownAudit<Events::Item, Engine::Output>
where
    Events: Iterator,
    Events::Item: Debug + Clone,
    Engine: Processor<Events::Item> + Auditor<Engine::Audit, Context = EngineContext>,
    Engine::Audit: From<Engine::Snapshot> + From<ShutdownAudit<Events::Item, Engine::Output>>,
    Engine::Output: Debug + Clone,
    AuditTx: Tx<Item = AuditTick<Engine::Audit, EngineContext>>,
    Option<ShutdownAudit<Events::Item, Engine::Output>>: for<'a> From<&'a Engine::Audit>,
{
    info!(feed_mode = "sync", audit_mode = "enabled", "Engine running");

    // Send initial Engine State snapshot
    audit_tx.send(engine.audit(engine.snapshot()));

    // Run Engine process loop until shutdown
    let shutdown_audit = loop {
        let Some(event) = feed.next() else {
            audit_tx.send(engine.audit(ShutdownAudit::FeedEnded));
            break ShutdownAudit::FeedEnded;
        };

        // Process Event with AuditTick generation
        let audit = process_with_audit(engine, event);

        // Check if AuditTick indicates shutdown is required
        let shutdown = Option::<ShutdownAudit<Events::Item, Engine::Output>>::from(&audit.event);

        // Send AuditTick to AuditManager
        audit_tx.send(audit);

        if let Some(shutdown) = shutdown {
            break shutdown;
        }
    };

    // Send Shutdown audit
    audit_tx.send(engine.audit(shutdown_audit.clone()));

    info!(?shutdown_audit, "Engine shutting down");
    shutdown_audit
}

/// Asynchronous `Engine` runner that processes input `Events`.
///
/// Runs until shutdown, returning a [`ShutdownAudit`] detailing the reason for the shutdown
/// (eg/ `Events` `FeedEnded`, `Command::Shutdown`, etc.).
///
/// # Arguments
/// * `Events` - `Stream` of events for the `Engine` to process.
/// * `Engine` - Event processor that produces audit events as output.
/// * `AuditTx` - Channel for sending produced audit events.
pub async fn async_run<Events, Engine>(
    feed: &mut Events,
    engine: &mut Engine,
) -> ShutdownAudit<Events::Item, Engine::Output>
where
    Events: Stream + Unpin,
    Events::Item: Debug + Clone,
    Engine: Processor<Events::Item> + Auditor<Engine::Audit, Context = EngineContext>,
    Engine::Audit: From<Engine::Snapshot> + From<ShutdownAudit<Events::Item, Engine::Output>>,
    Engine::Output: Debug + Clone,
    Option<ShutdownAudit<Events::Item, Engine::Output>>: for<'a> From<&'a Engine::Audit>,
{
    info!(
        feed_mode = "async",
        audit_mode = "disabled",
        "Engine running"
    );

    // Run Engine process loop until shutdown
    let shutdown_audit = loop {
        let Some(event) = feed.next().await else {
            break ShutdownAudit::FeedEnded;
        };

        // Process Event with AuditTick generation
        let audit = process_with_audit(engine, event);

        // Check if AuditTick indicates shutdown is required
        let shutdown = Option::<ShutdownAudit<Events::Item, Engine::Output>>::from(&audit.event);

        if let Some(shutdown) = shutdown {
            break shutdown;
        }
    };

    info!(?shutdown_audit, "Engine shutting down");
    shutdown_audit
}

/// Asynchronous `Engine` runner that processes input `Events` and forwards audits to the provided
/// `AuditTx`.
///
/// Runs until shutdown, returning a [`ShutdownAudit`] detailing the reason for the shutdown
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
) -> ShutdownAudit<Events::Item, Engine::Output>
where
    Events: Stream + Unpin,
    Events::Item: Debug + Clone,
    Engine: Processor<Events::Item> + Auditor<Engine::Audit, Context = EngineContext>,
    Engine::Audit: From<Engine::Snapshot> + From<ShutdownAudit<Events::Item, Engine::Output>>,
    Engine::Output: Debug + Clone,
    AuditTx: Tx<Item = AuditTick<Engine::Audit, EngineContext>>,
    Option<ShutdownAudit<Events::Item, Engine::Output>>: for<'a> From<&'a Engine::Audit>,
{
    info!(
        feed_mode = "async",
        audit_mode = "enabled",
        "Engine running"
    );

    // Send initial Engine State snapshot
    audit_tx.send(engine.audit(engine.snapshot()));

    // Run Engine process loop until shutdown
    let shutdown_audit = loop {
        let Some(event) = feed.next().await else {
            audit_tx.send(engine.audit(ShutdownAudit::FeedEnded));
            break ShutdownAudit::FeedEnded;
        };

        // Process Event with AuditTick generation
        let audit = process_with_audit(engine, event);

        // Check if AuditTick indicates shutdown is required
        let shutdown = Option::<ShutdownAudit<Events::Item, Engine::Output>>::from(&audit.event);

        // Send AuditTick to AuditManager
        audit_tx.send(audit);

        if let Some(shutdown) = shutdown {
            break shutdown;
        }
    };

    // Send Shutdown audit
    audit_tx.send(engine.audit(shutdown_audit.clone()));

    info!(?shutdown_audit, "Engine shutting down");
    shutdown_audit
}
