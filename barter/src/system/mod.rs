use crate::engine::audit::{
    AuditTick, EngineAudit, context::EngineContext, shutdown::ShutdownAudit,
};
use barter_execution::client::mock::MockExecutionConfig;
use barter_integration::channel::{UnboundedRx, UnboundedTx};
use std::fmt::Debug;
use tokio::task::JoinHandle;

pub mod builder;

#[derive(Debug)]
pub struct System<Engine, Event, Output, State> {
    pub handles: SystemHandles<Engine, Event, Output>,
    pub feed_tx: UnboundedTx<Event>,
    pub audit_rx: Option<UnboundedRx<AuditTick<EngineAudit<State, Event, Output>, EngineContext>>>,
}

#[derive(Debug)]
pub struct SystemHandles<Engine, Event, Output> {
    pub runtime: tokio::runtime::Handle,
    pub engine: JoinHandle<(Engine, ShutdownAudit<Event, Output>)>,
    pub market_to_engine: JoinHandle<()>,
    pub account_to_engine: JoinHandle<()>,
}

#[derive(Debug, Default)]
pub enum EngineFeedMode {
    #[default]
    Sync,
    Async,
}

#[derive(Debug, Default)]
pub enum AuditMode {
    Enabled,
    #[default]
    Disabled,
}

#[derive(Debug)]
pub enum ExecutionConfig {
    Mock(MockExecutionConfig),
}
