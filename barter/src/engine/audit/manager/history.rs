use crate::engine::{
    action::{generate_algo_orders::GenerateAlgoOrdersOutput, ActionOutput},
    audit::manager::AuditTick,
    state::position::PositionExited,
};
use barter_execution::trade::Trade;

#[derive(Debug, Clone)]
pub struct AuditHistory<State, OnDisable, OnDisconnect, ExchangeKey, AssetKey, InstrumentKey> {
    pub states: Vec<AuditTick<State>>,
    pub commands: Vec<AuditTick<ActionOutput<ExchangeKey, InstrumentKey>>>,
    pub trading_disables: Vec<AuditTick<OnDisable>>,
    pub disconnections: Vec<AuditTick<OnDisconnect>>,
    pub orders: Vec<AuditTick<GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>>>,
    pub trades: Vec<AuditTick<Trade<AssetKey, InstrumentKey>>>,
    pub positions: Vec<AuditTick<PositionExited<AssetKey, InstrumentKey>>>,
}

impl<State, OnDisable, OnDisconnect, ExchangeKey, AssetKey, InstrumentKey> Default
    for AuditHistory<State, OnDisable, OnDisconnect, ExchangeKey, AssetKey, InstrumentKey>
{
    fn default() -> Self {
        Self {
            states: vec![],
            commands: vec![],
            trading_disables: vec![],
            disconnections: vec![],
            orders: vec![],
            trades: vec![],
            positions: vec![],
        }
    }
}
