use crate::{
    processor::ProcessedSignal,
    Result, StrategyError,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingDecision {
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub action: TradingAction,
    pub confidence: f64,
    pub risk_score: f64,
    pub rationale: String,
    pub target_price: Option<Decimal>,
    pub stop_loss: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TradingAction {
    OpenLong,
    OpenShort,
    CloseLong,
    CloseShort,
    IncreasePosition,
    ReducePosition,
    Hold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketCondition {
    pub trend: TrendDirection,
    pub strength: f64,
    pub volatility_regime: VolatilityRegime,
    pub market_phase: MarketPhase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Bullish,
    Bearish,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VolatilityRegime {
    Low,
    Normal,
    High,
    Extreme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketPhase {
    Accumulation,
    Markup,
    Distribution,
    Decline,
}

pub struct SignalJudgment {
    model_path: Option<String>,
    risk_threshold: f64,
    confidence_threshold: f64,
}

impl SignalJudgment {
    pub fn new(risk_threshold: f64, confidence_threshold: f64) -> Self {
        Self {
            model_path: None,
            risk_threshold,
            confidence_threshold,
        }
    }

    pub fn with_model(mut self, model_path: String) -> Self {
        self.model_path = Some(model_path);
        self
    }

    pub async fn judge(&self, signal: ProcessedSignal) -> Result<TradingDecision> {
        info!("Judging processed signal for {}", signal.symbol);

        // Analyze market conditions
        let market_condition = self.analyze_market_condition(&signal)?;

        // Make trading decision based on conditions and indicators
        let decision = self.make_decision(&signal, &market_condition)?;

        // Validate decision against risk parameters
        let validated_decision = self.validate_decision(decision)?;

        Ok(validated_decision)
    }

    fn analyze_market_condition(&self, signal: &ProcessedSignal) -> Result<MarketCondition> {
        // Determine trend direction
        let trend = self.determine_trend(&signal.indicators);

        // Calculate trend strength
        let strength = self.calculate_trend_strength(&signal.indicators);

        // Classify volatility regime
        let volatility_regime = match signal.features.volatility {
            v if v < 0.15 => VolatilityRegime::Low,
            v if v < 0.30 => VolatilityRegime::Normal,
            v if v < 0.50 => VolatilityRegime::High,
            _ => VolatilityRegime::Extreme,
        };

        // Determine market phase
        let market_phase = self.determine_market_phase(&signal.indicators, &trend);

        Ok(MarketCondition {
            trend,
            strength,
            volatility_regime,
            market_phase,
        })
    }

    fn determine_trend(&self, indicators: &crate::processor::TechnicalIndicators) -> TrendDirection {
        let mut bullish_signals = 0;
        let mut bearish_signals = 0;

        // Check moving averages
        if let (Some(sma20), Some(sma50)) = (indicators.sma_20, indicators.sma_50) {
            if sma20 > sma50 {
                bullish_signals += 1;
            } else {
                bearish_signals += 1;
            }
        }

        // Check MACD
        if let Some(macd) = indicators.macd {
            if macd > 0.0 {
                bullish_signals += 1;
            } else {
                bearish_signals += 1;
            }
        }

        // Check RSI
        if let Some(rsi) = indicators.rsi {
            if rsi > 50.0 {
                bullish_signals += 1;
            } else if rsi < 50.0 {
                bearish_signals += 1;
            }
        }

        if bullish_signals > bearish_signals {
            TrendDirection::Bullish
        } else if bearish_signals > bullish_signals {
            TrendDirection::Bearish
        } else {
            TrendDirection::Neutral
        }
    }

    fn calculate_trend_strength(&self, indicators: &crate::processor::TechnicalIndicators) -> f64 {
        let mut strength = 0.0;
        let mut count = 0;

        // Use RSI deviation from 50
        if let Some(rsi) = indicators.rsi {
            strength += ((rsi - 50.0).abs() / 50.0);
            count += 1;
        }

        // Use MACD strength
        if let Some(macd) = indicators.macd {
            strength += (macd.abs().min(1.0));
            count += 1;
        }

        // Use moving average separation
        if let (Some(sma20), Some(sma50)) = (indicators.sma_20, indicators.sma_50) {
            let separation = ((sma20 - sma50) / sma50).abs();
            strength += separation.min(1.0);
            count += 1;
        }

        if count > 0 {
            strength / count as f64
        } else {
            0.0
        }
    }

    fn determine_market_phase(
        &self,
        indicators: &crate::processor::TechnicalIndicators,
        trend: &TrendDirection,
    ) -> MarketPhase {
        match trend {
            TrendDirection::Bullish => {
                if let Some(rsi) = indicators.rsi {
                    if rsi < 30.0 {
                        MarketPhase::Accumulation
                    } else if rsi > 70.0 {
                        MarketPhase::Distribution
                    } else {
                        MarketPhase::Markup
                    }
                } else {
                    MarketPhase::Markup
                }
            }
            TrendDirection::Bearish => {
                if let Some(rsi) = indicators.rsi {
                    if rsi > 70.0 {
                        MarketPhase::Distribution
                    } else {
                        MarketPhase::Decline
                    }
                } else {
                    MarketPhase::Decline
                }
            }
            TrendDirection::Neutral => MarketPhase::Accumulation,
        }
    }

    fn make_decision(
        &self,
        signal: &ProcessedSignal,
        condition: &MarketCondition,
    ) -> Result<TradingDecision> {
        let mut action = TradingAction::Hold;
        let mut confidence = 0.5;
        let mut rationale = String::new();

        // Decision logic based on market conditions
        match (&condition.trend, &condition.market_phase) {
            (TrendDirection::Bullish, MarketPhase::Accumulation) => {
                action = TradingAction::OpenLong;
                confidence = 0.7 + condition.strength * 0.3;
                rationale = "Bullish trend in accumulation phase - good entry for long".to_string();
            }
            (TrendDirection::Bullish, MarketPhase::Markup) => {
                if condition.strength > 0.6 {
                    action = TradingAction::IncreasePosition;
                    confidence = 0.6 + condition.strength * 0.2;
                    rationale = "Strong bullish momentum - increase long position".to_string();
                }
            }
            (TrendDirection::Bullish, MarketPhase::Distribution) => {
                action = TradingAction::CloseLong;
                confidence = 0.7;
                rationale = "Potential top forming - close long positions".to_string();
            }
            (TrendDirection::Bearish, MarketPhase::Distribution) => {
                action = TradingAction::OpenShort;
                confidence = 0.7 + condition.strength * 0.3;
                rationale = "Bearish reversal starting - open short position".to_string();
            }
            (TrendDirection::Bearish, MarketPhase::Decline) => {
                if condition.strength > 0.6 {
                    action = TradingAction::IncreasePosition;
                    confidence = 0.6 + condition.strength * 0.2;
                    rationale = "Strong bearish momentum - increase short position".to_string();
                }
            }
            _ => {
                action = TradingAction::Hold;
                confidence = 0.3;
                rationale = "Unclear market conditions - hold current position".to_string();
            }
        }

        // Check for oversold/overbought conditions
        if let Some(rsi) = signal.indicators.rsi {
            if rsi < 30.0 && action != TradingAction::OpenLong {
                action = TradingAction::OpenLong;
                confidence = 0.8;
                rationale = format!("RSI oversold at {:.2} - potential reversal", rsi);
            } else if rsi > 70.0 && action != TradingAction::OpenShort {
                action = TradingAction::OpenShort;
                confidence = 0.8;
                rationale = format!("RSI overbought at {:.2} - potential reversal", rsi);
            }
        }

        // Calculate risk score
        let risk_score = self.calculate_risk_score(signal, condition);

        // Set target and stop loss
        let (target_price, stop_loss) = self.calculate_price_targets(signal, &action);

        Ok(TradingDecision {
            timestamp: signal.timestamp,
            symbol: signal.symbol.clone(),
            action,
            confidence,
            risk_score,
            rationale,
            target_price,
            stop_loss,
        })
    }

    fn calculate_risk_score(
        &self,
        signal: &ProcessedSignal,
        condition: &MarketCondition,
    ) -> f64 {
        let mut risk = 0.0;

        // Volatility risk
        risk += match condition.volatility_regime {
            VolatilityRegime::Low => 0.2,
            VolatilityRegime::Normal => 0.4,
            VolatilityRegime::High => 0.7,
            VolatilityRegime::Extreme => 1.0,
        };

        // Spread risk
        let spread_pct = signal.market_microstructure.bid_ask_spread / signal.features.price;
        risk += spread_pct.to_string().parse::<f64>().unwrap_or(0.0).min(0.5);

        // Liquidity risk
        if signal.market_microstructure.trade_intensity < 0.1 {
            risk += 0.3;
        }

        risk.min(1.0)
    }

    fn calculate_price_targets(
        &self,
        signal: &ProcessedSignal,
        action: &TradingAction,
    ) -> (Option<Decimal>, Option<Decimal>) {
        let current_price = signal.features.price;
        let atr_pct = Decimal::from_f64_retain(signal.features.volatility).unwrap_or(Decimal::from_str_exact("0.02").unwrap());

        match action {
            TradingAction::OpenLong | TradingAction::IncreasePosition => {
                let target = current_price * (Decimal::ONE + atr_pct * Decimal::from(2));
                let stop = current_price * (Decimal::ONE - atr_pct);
                (Some(target), Some(stop))
            }
            TradingAction::OpenShort => {
                let target = current_price * (Decimal::ONE - atr_pct * Decimal::from(2));
                let stop = current_price * (Decimal::ONE + atr_pct);
                (Some(target), Some(stop))
            }
            _ => (None, None),
        }
    }

    fn validate_decision(&self, mut decision: TradingDecision) -> Result<TradingDecision> {
        // Check confidence threshold
        if decision.confidence < self.confidence_threshold {
            debug!(
                "Decision confidence {} below threshold {}, changing to HOLD",
                decision.confidence, self.confidence_threshold
            );
            decision.action = TradingAction::Hold;
            decision.rationale = format!(
                "Confidence too low ({:.2}%). {}",
                decision.confidence * 100.0,
                decision.rationale
            );
        }

        // Check risk threshold
        if decision.risk_score > self.risk_threshold {
            debug!(
                "Risk score {} above threshold {}, changing to HOLD",
                decision.risk_score, self.risk_threshold
            );
            decision.action = TradingAction::Hold;
            decision.rationale = format!(
                "Risk too high ({:.2}). {}",
                decision.risk_score,
                decision.rationale
            );
        }

        Ok(decision)
    }
}

#[async_trait]
pub trait AIModel {
    async fn predict(&self, features: Vec<f64>) -> Result<TradingDecision>;
    async fn update(&mut self, feedback: TradingFeedback) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingFeedback {
    pub decision: TradingDecision,
    pub actual_price: Decimal,
    pub pnl: Decimal,
    pub success: bool,
}