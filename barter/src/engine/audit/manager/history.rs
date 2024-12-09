use crate::engine::{
    action::{generate_algo_orders::GenerateAlgoOrdersOutput, ActionOutput},
    audit::manager::AuditTick,
    state::position::PositionExited,
};
use barter_execution::trade::Trade;
use barter_instrument::asset::QuoteAsset;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct TradingHistory<State, OnDisable, OnDisconnect, ExchangeKey, InstrumentKey> {
    pub states: Vec<AuditTick<State>>,
    pub commands: Vec<AuditTick<ActionOutput<ExchangeKey, InstrumentKey>>>,
    pub trading_disables: Vec<AuditTick<OnDisable>>,
    pub disconnections: Vec<AuditTick<OnDisconnect>>,
    pub orders_sent: Vec<AuditTick<GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>>>,
    pub trades: Vec<AuditTick<Trade<QuoteAsset, InstrumentKey>>>,
    pub positions: Vec<AuditTick<PositionExited<QuoteAsset, InstrumentKey>>>,
}

impl<State, OnDisable, OnDisconnect, ExchangeKey, InstrumentKey> From<AuditTick<State>>
    for TradingHistory<State, OnDisable, OnDisconnect, ExchangeKey, InstrumentKey>
{
    fn from(value: AuditTick<State>) -> Self {
        Self {
            states: vec![value],
            ..Default::default()
        }
    }
}

impl<State, OnDisable, OnDisconnect, ExchangeKey, InstrumentKey> Default
    for TradingHistory<State, OnDisable, OnDisconnect, ExchangeKey, InstrumentKey>
{
    fn default() -> Self {
        Self {
            states: vec![],
            commands: vec![],
            trading_disables: vec![],
            disconnections: vec![],
            orders_sent: vec![],
            trades: vec![],
            positions: vec![],
        }
    }
}
