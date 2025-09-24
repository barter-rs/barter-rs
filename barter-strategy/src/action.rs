use crate::{
    judgment::{TradingAction, TradingDecision},
    Result, StrategyError,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyAction {
    pub action_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub quantity: Decimal,
    pub price: Option<Decimal>,
    pub leverage: u8,
    pub reduce_only: bool,
    pub time_in_force: TimeInForce,
    pub stop_loss: Option<Decimal>,
    pub take_profit: Option<Decimal>,
    pub metadata: ActionMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    StopMarket,
    StopLimit,
    TakeProfitMarket,
    TakeProfitLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeInForce {
    GTC,  // Good Till Cancelled
    IOC,  // Immediate or Cancel
    FOK,  // Fill or Kill
    GTX,  // Good Till Crossing
    Post, // Post Only
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionMetadata {
    pub decision_id: String,
    pub confidence: f64,
    pub risk_score: f64,
    pub expected_pnl: Option<Decimal>,
    pub max_loss: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,
    pub side: PositionSide,
    pub quantity: Decimal,
    pub entry_price: Decimal,
    pub mark_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub margin: Decimal,
    pub leverage: u8,
    pub liquidation_price: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PositionSide {
    Long,
    Short,
    None,
}

pub struct ActionGenerator {
    positions: HashMap<String, Position>,
    risk_params: RiskParameters,
    capital: Decimal,
}

#[derive(Debug, Clone)]
pub struct RiskParameters {
    pub max_position_size: Decimal,
    pub max_leverage: u8,
    pub default_leverage: u8,
    pub position_sizing_method: PositionSizingMethod,
    pub max_loss_per_trade: Decimal,
    pub max_daily_loss: Decimal,
}

#[derive(Debug, Clone)]
pub enum PositionSizingMethod {
    Fixed(Decimal),
    PercentageOfCapital(Decimal),
    Kelly,
    RiskParity,
}

impl Default for RiskParameters {
    fn default() -> Self {
        Self {
            max_position_size: Decimal::from(10000),
            max_leverage: 20,
            default_leverage: 5,
            position_sizing_method: PositionSizingMethod::PercentageOfCapital(Decimal::from_str_exact("0.1").unwrap()),
            max_loss_per_trade: Decimal::from_str_exact("0.02").unwrap(),
            max_daily_loss: Decimal::from_str_exact("0.05").unwrap(),
        }
    }
}

impl ActionGenerator {
    pub fn new(capital: Decimal, risk_params: RiskParameters) -> Self {
        Self {
            positions: HashMap::new(),
            risk_params,
            capital,
        }
    }

    pub async fn generate_action(&mut self, decision: TradingDecision) -> Result<Option<StrategyAction>> {
        info!("Generating action for decision: {:?}", decision.action);

        // Check if we should take action
        if decision.action == TradingAction::Hold {
            debug!("Decision is HOLD, no action generated");
            return Ok(None);
        }

        // Get current position for the symbol
        let current_position = self.positions.get(&decision.symbol);

        // Generate appropriate action based on decision and current position
        let action = match decision.action {
            TradingAction::OpenLong => {
                self.generate_open_position(decision, OrderSide::Buy)?
            }
            TradingAction::OpenShort => {
                self.generate_open_position(decision, OrderSide::Sell)?
            }
            TradingAction::CloseLong => {
                self.generate_close_position(decision, PositionSide::Long)?
            }
            TradingAction::CloseShort => {
                self.generate_close_position(decision, PositionSide::Short)?
            }
            TradingAction::IncreasePosition => {
                self.generate_adjust_position(decision, true)?
            }
            TradingAction::ReducePosition => {
                self.generate_adjust_position(decision, false)?
            }
            TradingAction::Hold => return Ok(None),
        };

        Ok(action)
    }

    fn generate_open_position(
        &self,
        decision: TradingDecision,
        side: OrderSide,
    ) -> Result<Option<StrategyAction>> {
        // Check if we already have a position
        if let Some(position) = self.positions.get(&decision.symbol) {
            if (position.side == PositionSide::Long && side == OrderSide::Buy)
                || (position.side == PositionSide::Short && side == OrderSide::Sell)
            {
                warn!("Already have a {} position for {}", position.side.to_string(), decision.symbol);
                return Ok(None);
            }
        }

        // Calculate position size
        let quantity = self.calculate_position_size(&decision)?;

        // Create the action
        let action = StrategyAction {
            action_id: format!("action_{}", Utc::now().timestamp_nanos_opt().unwrap()),
            timestamp: decision.timestamp,
            symbol: decision.symbol.clone(),
            order_type: OrderType::Market,
            side,
            quantity,
            price: None,
            leverage: self.risk_params.default_leverage,
            reduce_only: false,
            time_in_force: TimeInForce::GTC,
            stop_loss: decision.stop_loss,
            take_profit: decision.target_price,
            metadata: ActionMetadata {
                decision_id: format!("decision_{}", decision.timestamp.timestamp_nanos_opt().unwrap()),
                confidence: decision.confidence,
                risk_score: decision.risk_score,
                expected_pnl: self.calculate_expected_pnl(&decision, quantity),
                max_loss: self.calculate_max_loss(&decision, quantity),
            },
        };

        info!("Generated open position action: {:?}", action.order_type);
        Ok(Some(action))
    }

    fn generate_close_position(
        &self,
        decision: TradingDecision,
        position_side: PositionSide,
    ) -> Result<Option<StrategyAction>> {
        // Check if we have a position to close
        let position = match self.positions.get(&decision.symbol) {
            Some(pos) if pos.side == position_side => pos,
            _ => {
                warn!("No {} position to close for {}", position_side.to_string(), decision.symbol);
                return Ok(None);
            }
        };

        let side = match position_side {
            PositionSide::Long => OrderSide::Sell,
            PositionSide::Short => OrderSide::Buy,
            PositionSide::None => return Ok(None),
        };

        let action = StrategyAction {
            action_id: format!("action_{}", Utc::now().timestamp_nanos_opt().unwrap()),
            timestamp: decision.timestamp,
            symbol: decision.symbol.clone(),
            order_type: OrderType::Market,
            side,
            quantity: position.quantity,
            price: None,
            leverage: position.leverage,
            reduce_only: true,
            time_in_force: TimeInForce::GTC,
            stop_loss: None,
            take_profit: None,
            metadata: ActionMetadata {
                decision_id: format!("decision_{}", decision.timestamp.timestamp_nanos_opt().unwrap()),
                confidence: decision.confidence,
                risk_score: decision.risk_score,
                expected_pnl: Some(position.unrealized_pnl),
                max_loss: None,
            },
        };

        info!("Generated close position action");
        Ok(Some(action))
    }

    fn generate_adjust_position(
        &self,
        decision: TradingDecision,
        increase: bool,
    ) -> Result<Option<StrategyAction>> {
        // Check if we have a position to adjust
        let position = match self.positions.get(&decision.symbol) {
            Some(pos) => pos,
            None => {
                warn!("No position to adjust for {}", decision.symbol);
                return Ok(None);
            }
        };

        // Calculate adjustment quantity
        let adjustment_qty = if increase {
            self.calculate_position_size(&decision)? * Decimal::from_str_exact("0.5").unwrap()
        } else {
            position.quantity * Decimal::from_str_exact("0.3").unwrap()
        };

        let side = if increase {
            match position.side {
                PositionSide::Long => OrderSide::Buy,
                PositionSide::Short => OrderSide::Sell,
                PositionSide::None => return Ok(None),
            }
        } else {
            match position.side {
                PositionSide::Long => OrderSide::Sell,
                PositionSide::Short => OrderSide::Buy,
                PositionSide::None => return Ok(None),
            }
        };

        let action = StrategyAction {
            action_id: format!("action_{}", Utc::now().timestamp_nanos_opt().unwrap()),
            timestamp: decision.timestamp,
            symbol: decision.symbol.clone(),
            order_type: OrderType::Market,
            side,
            quantity: adjustment_qty,
            price: None,
            leverage: position.leverage,
            reduce_only: !increase,
            time_in_force: TimeInForce::GTC,
            stop_loss: decision.stop_loss,
            take_profit: decision.target_price,
            metadata: ActionMetadata {
                decision_id: format!("decision_{}", decision.timestamp.timestamp_nanos_opt().unwrap()),
                confidence: decision.confidence,
                risk_score: decision.risk_score,
                expected_pnl: None,
                max_loss: None,
            },
        };

        info!("Generated position adjustment action");
        Ok(Some(action))
    }

    fn calculate_position_size(&self, decision: &TradingDecision) -> Result<Decimal> {
        let size = match &self.risk_params.position_sizing_method {
            PositionSizingMethod::Fixed(size) => *size,
            PositionSizingMethod::PercentageOfCapital(pct) => self.capital * pct,
            PositionSizingMethod::Kelly => {
                // Simplified Kelly criterion
                let win_prob = decision.confidence;
                let loss_prob = 1.0 - win_prob;
                let win_loss_ratio = 2.0; // Assume 2:1 reward/risk

                let kelly_pct = (win_prob * win_loss_ratio - loss_prob) / win_loss_ratio;
                let kelly_decimal = Decimal::from_f64_retain(kelly_pct.max(0.0).min(0.25))
                    .unwrap_or(Decimal::ZERO);

                self.capital * kelly_decimal
            }
            PositionSizingMethod::RiskParity => {
                // Risk parity sizing based on volatility
                let vol_adjusted = Decimal::from_f64_retain(1.0 / (1.0 + decision.risk_score))
                    .unwrap_or(Decimal::ONE);
                self.capital * Decimal::from_str_exact("0.1").unwrap() * vol_adjusted
            }
        };

        // Apply maximum position size constraint
        Ok(size.min(self.risk_params.max_position_size))
    }

    fn calculate_expected_pnl(
        &self,
        decision: &TradingDecision,
        quantity: Decimal,
    ) -> Option<Decimal> {
        decision.target_price.map(|target| {
            // This is a simplified calculation
            // In reality, we'd need the current price and consider leverage
            quantity * Decimal::from_str_exact("0.05").unwrap() // Assume 5% profit target
        })
    }

    fn calculate_max_loss(
        &self,
        decision: &TradingDecision,
        quantity: Decimal,
    ) -> Option<Decimal> {
        decision.stop_loss.map(|_| {
            // Maximum loss based on stop loss
            quantity * self.risk_params.max_loss_per_trade
        })
    }

    pub fn update_position(&mut self, symbol: String, position: Position) {
        self.positions.insert(symbol, position);
    }

    pub fn remove_position(&mut self, symbol: &str) {
        self.positions.remove(symbol);
    }

    pub fn get_position(&self, symbol: &str) -> Option<&Position> {
        self.positions.get(symbol)
    }
}

impl PositionSide {
    fn to_string(&self) -> &str {
        match self {
            PositionSide::Long => "long",
            PositionSide::Short => "short",
            PositionSide::None => "none",
        }
    }
}