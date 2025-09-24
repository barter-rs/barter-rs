use crate::{
    action::{OrderSide, OrderType, StrategyAction, TimeInForce},
    Result, StrategyError,
};
use async_trait::async_trait;
use barter_execution::{
    balance::BalanceKind,
    client::{ClientKind, ExecutionClient},
    error::ExecutionError,
    order::{Order, OrderId, OrderKind, OrderStatus, RequestCancel, RequestOpen},
    trade::Trade,
};
use barter_instrument::exchange::ExchangeId;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub action_id: String,
    pub order_id: String,
    pub status: ExecutionStatus,
    pub filled_quantity: Decimal,
    pub average_price: Option<Decimal>,
    pub commission: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    Pending,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
    Failed,
}

pub struct StrategyExecution {
    exchange_id: ExchangeId,
    test_mode: bool,
    orders: HashMap<String, OrderTracking>,
}

#[derive(Debug, Clone)]
struct OrderTracking {
    action: StrategyAction,
    order_id: Option<OrderId>,
    status: ExecutionStatus,
    filled_quantity: Decimal,
    average_price: Option<Decimal>,
}

impl StrategyExecution {
    pub fn new(exchange_id: ExchangeId, test_mode: bool) -> Self {
        Self {
            exchange_id,
            test_mode,
            orders: HashMap::new(),
        }
    }

    pub async fn execute(&mut self, action: StrategyAction) -> Result<ExecutionResult> {
        info!("Executing strategy action: {:?}", action.action_id);

        if self.test_mode {
            return self.execute_test_mode(action).await;
        }

        // In production mode, this would interface with real exchange
        // For now, we'll create a simulated execution
        self.execute_simulated(action).await
    }

    async fn execute_test_mode(&mut self, action: StrategyAction) -> Result<ExecutionResult> {
        // Test mode execution - always succeeds
        info!("Test mode execution for action: {}", action.action_id);

        let result = ExecutionResult {
            action_id: action.action_id.clone(),
            order_id: format!("test_order_{}", Utc::now().timestamp_nanos_opt().unwrap()),
            status: ExecutionStatus::Filled,
            filled_quantity: action.quantity,
            average_price: action.price.or(Some(Decimal::from(100))), // Use provided price or default
            commission: Some(action.quantity * Decimal::from_str_exact("0.0004").unwrap()), // 0.04% commission
            timestamp: Utc::now(),
            error: None,
        };

        Ok(result)
    }

    async fn execute_simulated(&mut self, action: StrategyAction) -> Result<ExecutionResult> {
        // Simulated execution for development
        info!("Simulated execution for action: {}", action.action_id);

        // Track the order
        let tracking = OrderTracking {
            action: action.clone(),
            order_id: None,
            status: ExecutionStatus::Pending,
            filled_quantity: Decimal::ZERO,
            average_price: None,
        };
        self.orders.insert(action.action_id.clone(), tracking);

        // Simulate order placement
        let order_id = format!("sim_order_{}", Utc::now().timestamp_nanos_opt().unwrap());

        // Simulate successful fill after validation
        let (status, filled_qty) = if action.quantity > Decimal::ZERO {
            (ExecutionStatus::Filled, action.quantity)
        } else {
            (ExecutionStatus::Rejected, Decimal::ZERO)
        };

        let average_price = if status == ExecutionStatus::Filled {
            action.price.or(Some(Decimal::from(100))) // Use action price or default
        } else {
            None
        };

        let result = ExecutionResult {
            action_id: action.action_id,
            order_id,
            status,
            filled_quantity: filled_qty,
            average_price,
            commission: if status == ExecutionStatus::Filled {
                Some(filled_qty * Decimal::from_str_exact("0.0004").unwrap())
            } else {
                None
            },
            timestamp: Utc::now(),
            error: if status == ExecutionStatus::Rejected {
                Some("Invalid order parameters".to_string())
            } else {
                None
            },
        };

        Ok(result)
    }

    pub async fn cancel_order(&mut self, action_id: &str) -> Result<ExecutionResult> {
        info!("Cancelling order for action: {}", action_id);

        if let Some(tracking) = self.orders.get_mut(action_id) {
            tracking.status = ExecutionStatus::Cancelled;

            Ok(ExecutionResult {
                action_id: action_id.to_string(),
                order_id: tracking
                    .order_id
                    .as_ref()
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                status: ExecutionStatus::Cancelled,
                filled_quantity: tracking.filled_quantity,
                average_price: tracking.average_price,
                commission: None,
                timestamp: Utc::now(),
                error: None,
            })
        } else {
            Err(StrategyError::Execution(format!(
                "Order {} not found",
                action_id
            )))
        }
    }

    pub async fn get_order_status(&self, action_id: &str) -> Result<ExecutionResult> {
        if let Some(tracking) = self.orders.get(action_id) {
            Ok(ExecutionResult {
                action_id: action_id.to_string(),
                order_id: tracking
                    .order_id
                    .as_ref()
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                status: tracking.status.clone(),
                filled_quantity: tracking.filled_quantity,
                average_price: tracking.average_price,
                commission: None,
                timestamp: Utc::now(),
                error: None,
            })
        } else {
            Err(StrategyError::Execution(format!(
                "Order {} not found",
                action_id
            )))
        }
    }

    pub async fn get_active_orders(&self) -> Vec<String> {
        self.orders
            .iter()
            .filter(|(_, tracking)| {
                matches!(
                    tracking.status,
                    ExecutionStatus::Pending | ExecutionStatus::PartiallyFilled
                )
            })
            .map(|(id, _)| id.clone())
            .collect()
    }
}

// Converter to map our OrderType to barter-execution OrderKind
fn convert_order_type(order_type: &OrderType, price: Option<Decimal>) -> OrderKind {
    match order_type {
        OrderType::Market => OrderKind::Market,
        OrderType::Limit => OrderKind::Limit {
            price: price.unwrap_or(Decimal::ZERO),
            post_only: false,
        },
        OrderType::StopMarket | OrderType::StopLimit => OrderKind::Market, // Simplified
        OrderType::TakeProfitMarket | OrderType::TakeProfitLimit => OrderKind::Market, // Simplified
    }
}

#[async_trait]
pub trait ExecutionEngine {
    async fn place_order(&mut self, action: StrategyAction) -> Result<ExecutionResult>;
    async fn cancel_order(&mut self, order_id: &str) -> Result<ExecutionResult>;
    async fn get_order_status(&self, order_id: &str) -> Result<ExecutionResult>;
    async fn get_positions(&self) -> Result<Vec<Position>>;
    async fn get_balance(&self) -> Result<Balance>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub side: String,
    pub quantity: Decimal,
    pub entry_price: Decimal,
    pub mark_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub margin: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub total: Decimal,
    pub free: Decimal,
    pub used: Decimal,
}