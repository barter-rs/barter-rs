use crate::{
    action::{ActionGenerator, OrderSide, Position, PositionSide, RiskParameters},
    execution::{ExecutionResult, ExecutionStatus, StrategyExecution},
    judgment::{SignalJudgment, TradingAction, TradingDecision},
    processor::{ProcessedSignal, SignalProcessor},
    signal::{MarketSignal, SignalData, TradeSide},
    Result, StrategyError,
};
use chrono::{DateTime, Duration, Utc};
use csv::{Reader, Writer};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestConfig {
    pub initial_capital: Decimal,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub symbol: String,
    pub risk_params: RiskParameters,
    pub commission_rate: Decimal,
    pub slippage_rate: Decimal,
    pub data_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub total_return: Decimal,
    pub max_drawdown: Decimal,
    pub sharpe_ratio: f64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub trades: Vec<TradeRecord>,
    pub equity_curve: Vec<EquityPoint>,
    pub statistics: BacktestStatistics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub entry_time: DateTime<Utc>,
    pub exit_time: Option<DateTime<Utc>>,
    pub symbol: String,
    pub side: String,
    pub entry_price: Decimal,
    pub exit_price: Option<Decimal>,
    pub quantity: Decimal,
    pub pnl: Option<Decimal>,
    pub return_pct: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityPoint {
    pub timestamp: DateTime<Utc>,
    pub equity: Decimal,
    pub cash: Decimal,
    pub positions_value: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestStatistics {
    pub avg_win: Decimal,
    pub avg_loss: Decimal,
    pub max_consecutive_wins: u32,
    pub max_consecutive_losses: u32,
    pub avg_trade_duration: Duration,
    pub best_trade: Decimal,
    pub worst_trade: Decimal,
}

pub struct Backtester {
    config: BacktestConfig,
    signal_processor: SignalProcessor,
    signal_judgment: SignalJudgment,
    action_generator: ActionGenerator,
    execution: StrategyExecution,
    trades: Vec<TradeRecord>,
    equity_curve: Vec<EquityPoint>,
    current_capital: Decimal,
    current_positions: HashMap<String, BacktestPosition>,
}

#[derive(Debug, Clone)]
struct BacktestPosition {
    entry_time: DateTime<Utc>,
    entry_price: Decimal,
    quantity: Decimal,
    side: PositionSide,
    unrealized_pnl: Decimal,
}

impl Backtester {
    pub fn new(config: BacktestConfig) -> Self {
        let signal_processor = SignalProcessor::new(config.symbol.clone(), 100);
        let signal_judgment = SignalJudgment::new(0.7, 0.6);
        let action_generator = ActionGenerator::new(config.initial_capital, config.risk_params.clone());
        let execution = StrategyExecution::new(barter_instrument::exchange::ExchangeId::BinanceFuturesUsd, true);

        Self {
            current_capital: config.initial_capital,
            config,
            signal_processor,
            signal_judgment,
            action_generator,
            execution,
            trades: Vec::new(),
            equity_curve: Vec::new(),
            current_positions: HashMap::new(),
        }
    }

    pub async fn run(&mut self) -> Result<BacktestResult> {
        info!("Starting backtest from {} to {}", self.config.start_date, self.config.end_date);

        // Load historical data
        let market_data = self.load_historical_data(&self.config.data_path)?;

        // Process each market signal
        for signal in market_data {
            if signal.timestamp < self.config.start_date || signal.timestamp > self.config.end_date {
                continue;
            }

            // Process signal
            let processed = self.signal_processor.process(signal.clone()).await?;

            // Make trading decision
            let decision = self.signal_judgment.judge(processed).await?;

            // Generate action
            if let Some(action) = self.action_generator.generate_action(decision.clone()).await? {
                // Execute action (simulated)
                let result = self.execution.execute(action.clone()).await?;

                // Update positions and capital
                self.update_from_execution(result, decision)?;
            }

            // Update equity curve
            self.update_equity_curve(signal.timestamp)?;

            // Update unrealized PnL for open positions
            self.update_unrealized_pnl(&signal)?;
        }

        // Close all remaining positions
        self.close_all_positions().await?;

        // Calculate statistics
        let statistics = self.calculate_statistics();

        Ok(BacktestResult {
            total_trades: self.trades.len() as u32,
            winning_trades: self.count_winning_trades(),
            losing_trades: self.count_losing_trades(),
            total_return: self.calculate_total_return(),
            max_drawdown: self.calculate_max_drawdown(),
            sharpe_ratio: self.calculate_sharpe_ratio(),
            win_rate: self.calculate_win_rate(),
            profit_factor: self.calculate_profit_factor(),
            trades: self.trades.clone(),
            equity_curve: self.equity_curve.clone(),
            statistics,
        })
    }

    fn load_historical_data(&self, path: &str) -> Result<Vec<MarketSignal>> {
        // In production, this would load from CSV or database
        // For now, generate synthetic data
        info!("Loading historical data from {}", path);

        let mut signals = Vec::new();
        let mut current_time = self.config.start_date;
        let mut price = Decimal::from(100);

        while current_time <= self.config.end_date {
            // Generate synthetic price movement
            let change = Decimal::from_f64_retain((rand::random::<f64>() - 0.5) * 2.0)
                .unwrap_or(Decimal::ZERO);
            price = price + change;

            signals.push(MarketSignal {
                timestamp: current_time,
                exchange: barter_instrument::exchange::ExchangeId::BinanceFuturesUsd,
                symbol: self.config.symbol.clone(),
                signal_type: crate::signal::SignalType::Trade,
                data: SignalData::Trade {
                    price,
                    amount: Decimal::from(10),
                    side: if rand::random::<bool>() {
                        TradeSide::Buy
                    } else {
                        TradeSide::Sell
                    },
                },
            });

            current_time = current_time + Duration::minutes(1);
        }

        Ok(signals)
    }

    fn update_from_execution(&mut self, result: ExecutionResult, decision: TradingDecision) -> Result<()> {
        if result.status != ExecutionStatus::Filled {
            return Ok(());
        }

        let price = result.average_price.unwrap_or(Decimal::from(100));
        let commission = result.commission.unwrap_or(Decimal::ZERO);

        match decision.action {
            TradingAction::OpenLong => {
                let position = BacktestPosition {
                    entry_time: decision.timestamp,
                    entry_price: price,
                    quantity: result.filled_quantity,
                    side: PositionSide::Long,
                    unrealized_pnl: Decimal::ZERO,
                };
                self.current_positions.insert(decision.symbol.clone(), position);
                self.current_capital -= (price * result.filled_quantity) + commission;

                self.trades.push(TradeRecord {
                    entry_time: decision.timestamp,
                    exit_time: None,
                    symbol: decision.symbol,
                    side: "long".to_string(),
                    entry_price: price,
                    exit_price: None,
                    quantity: result.filled_quantity,
                    pnl: None,
                    return_pct: None,
                });
            }
            TradingAction::OpenShort => {
                let position = BacktestPosition {
                    entry_time: decision.timestamp,
                    entry_price: price,
                    quantity: result.filled_quantity,
                    side: PositionSide::Short,
                    unrealized_pnl: Decimal::ZERO,
                };
                self.current_positions.insert(decision.symbol.clone(), position);

                self.trades.push(TradeRecord {
                    entry_time: decision.timestamp,
                    exit_time: None,
                    symbol: decision.symbol,
                    side: "short".to_string(),
                    entry_price: price,
                    exit_price: None,
                    quantity: result.filled_quantity,
                    pnl: None,
                    return_pct: None,
                });
            }
            TradingAction::CloseLong | TradingAction::CloseShort => {
                if let Some(position) = self.current_positions.remove(&decision.symbol) {
                    let pnl = self.calculate_pnl(&position, price);
                    self.current_capital += pnl - commission;

                    // Update the last trade record
                    if let Some(trade) = self.trades.iter_mut().rev().find(|t| t.symbol == decision.symbol && t.exit_time.is_none()) {
                        trade.exit_time = Some(decision.timestamp);
                        trade.exit_price = Some(price);
                        trade.pnl = Some(pnl);
                        trade.return_pct = Some(pnl / (position.entry_price * position.quantity));
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn calculate_pnl(&self, position: &BacktestPosition, exit_price: Decimal) -> Decimal {
        let price_diff = match position.side {
            PositionSide::Long => exit_price - position.entry_price,
            PositionSide::Short => position.entry_price - exit_price,
            PositionSide::None => Decimal::ZERO,
        };
        price_diff * position.quantity
    }

    fn update_unrealized_pnl(&mut self, signal: &MarketSignal) -> Result<()> {
        if let SignalData::Trade { price, .. } = &signal.data {
            for (_, position) in self.current_positions.iter_mut() {
                position.unrealized_pnl = self.calculate_pnl(position, *price);
            }
        }
        Ok(())
    }

    fn update_equity_curve(&mut self, timestamp: DateTime<Utc>) -> Result<()> {
        let positions_value: Decimal = self.current_positions.values()
            .map(|p| p.unrealized_pnl)
            .sum();

        let total_equity = self.current_capital + positions_value;

        self.equity_curve.push(EquityPoint {
            timestamp,
            equity: total_equity,
            cash: self.current_capital,
            positions_value,
        });

        Ok(())
    }

    async fn close_all_positions(&mut self) -> Result<()> {
        let positions: Vec<_> = self.current_positions.keys().cloned().collect();
        for symbol in positions {
            if let Some(position) = self.current_positions.get(&symbol) {
                let decision = TradingDecision {
                    timestamp: self.config.end_date,
                    symbol: symbol.clone(),
                    action: if position.side == PositionSide::Long {
                        TradingAction::CloseLong
                    } else {
                        TradingAction::CloseShort
                    },
                    confidence: 1.0,
                    risk_score: 0.0,
                    rationale: "End of backtest".to_string(),
                    target_price: None,
                    stop_loss: None,
                };

                if let Some(action) = self.action_generator.generate_action(decision.clone()).await? {
                    let result = self.execution.execute(action).await?;
                    self.update_from_execution(result, decision)?;
                }
            }
        }
        Ok(())
    }

    fn calculate_statistics(&self) -> BacktestStatistics {
        let winning_trades: Vec<_> = self.trades.iter()
            .filter(|t| t.pnl.map(|p| p > Decimal::ZERO).unwrap_or(false))
            .collect();

        let losing_trades: Vec<_> = self.trades.iter()
            .filter(|t| t.pnl.map(|p| p < Decimal::ZERO).unwrap_or(false))
            .collect();

        let avg_win = if !winning_trades.is_empty() {
            winning_trades.iter()
                .filter_map(|t| t.pnl)
                .sum::<Decimal>() / Decimal::from(winning_trades.len())
        } else {
            Decimal::ZERO
        };

        let avg_loss = if !losing_trades.is_empty() {
            losing_trades.iter()
                .filter_map(|t| t.pnl)
                .sum::<Decimal>() / Decimal::from(losing_trades.len())
        } else {
            Decimal::ZERO
        };

        BacktestStatistics {
            avg_win,
            avg_loss,
            max_consecutive_wins: self.calculate_max_consecutive_wins(),
            max_consecutive_losses: self.calculate_max_consecutive_losses(),
            avg_trade_duration: self.calculate_avg_trade_duration(),
            best_trade: self.trades.iter().filter_map(|t| t.pnl).max().unwrap_or(Decimal::ZERO),
            worst_trade: self.trades.iter().filter_map(|t| t.pnl).min().unwrap_or(Decimal::ZERO),
        }
    }

    fn count_winning_trades(&self) -> u32 {
        self.trades.iter()
            .filter(|t| t.pnl.map(|p| p > Decimal::ZERO).unwrap_or(false))
            .count() as u32
    }

    fn count_losing_trades(&self) -> u32 {
        self.trades.iter()
            .filter(|t| t.pnl.map(|p| p < Decimal::ZERO).unwrap_or(false))
            .count() as u32
    }

    fn calculate_total_return(&self) -> Decimal {
        if self.config.initial_capital == Decimal::ZERO {
            return Decimal::ZERO;
        }
        (self.current_capital - self.config.initial_capital) / self.config.initial_capital
    }

    fn calculate_max_drawdown(&self) -> Decimal {
        let mut max_equity = Decimal::ZERO;
        let mut max_drawdown = Decimal::ZERO;

        for point in &self.equity_curve {
            if point.equity > max_equity {
                max_equity = point.equity;
            }
            let drawdown = (max_equity - point.equity) / max_equity;
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }

        max_drawdown
    }

    fn calculate_sharpe_ratio(&self) -> f64 {
        if self.equity_curve.len() < 2 {
            return 0.0;
        }

        let returns: Vec<f64> = self.equity_curve.windows(2)
            .map(|w| {
                let prev = w[0].equity.to_string().parse::<f64>().unwrap_or(0.0);
                let curr = w[1].equity.to_string().parse::<f64>().unwrap_or(0.0);
                if prev != 0.0 {
                    (curr - prev) / prev
                } else {
                    0.0
                }
            })
            .collect();

        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance = returns.iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>() / returns.len() as f64;
        let std_dev = variance.sqrt();

        if std_dev == 0.0 {
            0.0
        } else {
            mean / std_dev * (252.0_f64).sqrt() // Annualized
        }
    }

    fn calculate_win_rate(&self) -> f64 {
        if self.trades.is_empty() {
            return 0.0;
        }
        self.count_winning_trades() as f64 / self.trades.len() as f64
    }

    fn calculate_profit_factor(&self) -> f64 {
        let gross_profit = self.trades.iter()
            .filter_map(|t| t.pnl)
            .filter(|p| *p > Decimal::ZERO)
            .sum::<Decimal>();

        let gross_loss = self.trades.iter()
            .filter_map(|t| t.pnl)
            .filter(|p| *p < Decimal::ZERO)
            .map(|p| p.abs())
            .sum::<Decimal>();

        if gross_loss == Decimal::ZERO {
            return f64::INFINITY;
        }

        gross_profit.to_string().parse::<f64>().unwrap_or(0.0) /
            gross_loss.to_string().parse::<f64>().unwrap_or(1.0)
    }

    fn calculate_max_consecutive_wins(&self) -> u32 {
        let mut max_consecutive = 0;
        let mut current_consecutive = 0;

        for trade in &self.trades {
            if trade.pnl.map(|p| p > Decimal::ZERO).unwrap_or(false) {
                current_consecutive += 1;
                max_consecutive = max_consecutive.max(current_consecutive);
            } else {
                current_consecutive = 0;
            }
        }

        max_consecutive
    }

    fn calculate_max_consecutive_losses(&self) -> u32 {
        let mut max_consecutive = 0;
        let mut current_consecutive = 0;

        for trade in &self.trades {
            if trade.pnl.map(|p| p < Decimal::ZERO).unwrap_or(false) {
                current_consecutive += 1;
                max_consecutive = max_consecutive.max(current_consecutive);
            } else {
                current_consecutive = 0;
            }
        }

        max_consecutive
    }

    fn calculate_avg_trade_duration(&self) -> Duration {
        let total_duration: i64 = self.trades.iter()
            .filter_map(|t| {
                t.exit_time.map(|exit| (exit - t.entry_time).num_seconds())
            })
            .sum();

        let closed_trades = self.trades.iter()
            .filter(|t| t.exit_time.is_some())
            .count() as i64;

        if closed_trades == 0 {
            Duration::zero()
        } else {
            Duration::seconds(total_duration / closed_trades)
        }
    }

    pub fn export_results(&self, result: &BacktestResult, path: &str) -> Result<()> {
        let mut wtr = Writer::from_path(path)
            .map_err(|e| StrategyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        for trade in &result.trades {
            wtr.serialize(trade)
                .map_err(|e| StrategyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        }

        wtr.flush()
            .map_err(|e| StrategyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        info!("Backtest results exported to {}", path);
        Ok(())
    }
}