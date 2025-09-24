use crate::action::{PositionSizingMethod, RiskParameters};
use barter_instrument::exchange::ExchangeId;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub exchanges: Vec<ExchangeConfig>,
    pub trading: TradingConfig,
    pub risk: RiskConfig,
    pub model: ModelConfig,
    pub queue: QueueConfig,
    pub backtest: BacktestConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub exchange_id: String,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub test_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    pub symbols: Vec<String>,
    pub leverage: u8,
    pub order_type: String,
    pub time_in_force: String,
    pub enable_stop_loss: bool,
    pub enable_take_profit: bool,
    pub stop_loss_pct: Decimal,
    pub take_profit_pct: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    pub max_position_size: Decimal,
    pub max_leverage: u8,
    pub position_sizing_method: String,
    pub sizing_parameter: Decimal,
    pub max_loss_per_trade: Decimal,
    pub max_daily_loss: Decimal,
    pub confidence_threshold: f64,
    pub risk_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_name: String,
    pub model_path: Option<String>,
    pub use_gpu: bool,
    pub batch_size: usize,
    pub prediction_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub enabled: bool,
    pub fluvio_url: Option<String>,
    pub topics: Vec<String>,
    pub consumer_group: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestConfig {
    pub enabled: bool,
    pub start_date: String,
    pub end_date: String,
    pub initial_capital: Decimal,
    pub data_source: String,
    pub export_results: bool,
    pub results_path: String,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            exchanges: vec![ExchangeConfig {
                exchange_id: "binance_futures".to_string(),
                api_key: None,
                api_secret: None,
                test_mode: true,
            }],
            trading: TradingConfig {
                symbols: vec!["ASTER/USDT:USDT".to_string()],
                leverage: 5,
                order_type: "market".to_string(),
                time_in_force: "GTC".to_string(),
                enable_stop_loss: true,
                enable_take_profit: true,
                stop_loss_pct: Decimal::from_str_exact("0.02").unwrap(),
                take_profit_pct: Decimal::from_str_exact("0.04").unwrap(),
            },
            risk: RiskConfig {
                max_position_size: Decimal::from(10000),
                max_leverage: 20,
                position_sizing_method: "percentage_of_capital".to_string(),
                sizing_parameter: Decimal::from_str_exact("0.1").unwrap(),
                max_loss_per_trade: Decimal::from_str_exact("0.02").unwrap(),
                max_daily_loss: Decimal::from_str_exact("0.05").unwrap(),
                confidence_threshold: 0.6,
                risk_threshold: 0.7,
            },
            model: ModelConfig {
                model_name: "mistral-7b".to_string(),
                model_path: None,
                use_gpu: false,
                batch_size: 1,
                prediction_threshold: 0.6,
            },
            queue: QueueConfig {
                enabled: false,
                fluvio_url: None,
                topics: vec![
                    "market-data".to_string(),
                    "processed-signals".to_string(),
                    "trading-decisions".to_string(),
                    "execution-results".to_string(),
                ],
                consumer_group: "strategy-group".to_string(),
            },
            backtest: BacktestConfig {
                enabled: false,
                start_date: "2024-01-01T00:00:00Z".to_string(),
                end_date: "2024-12-31T23:59:59Z".to_string(),
                initial_capital: Decimal::from(10000),
                data_source: "historical_data.csv".to_string(),
                export_results: true,
                results_path: "backtest_results.csv".to_string(),
            },
        }
    }
}

impl StrategyConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&contents)?;
        Ok(config)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }

    pub fn to_risk_parameters(&self) -> RiskParameters {
        let position_sizing = match self.risk.position_sizing_method.as_str() {
            "fixed" => PositionSizingMethod::Fixed(self.risk.sizing_parameter),
            "percentage_of_capital" => {
                PositionSizingMethod::PercentageOfCapital(self.risk.sizing_parameter)
            }
            "kelly" => PositionSizingMethod::Kelly,
            "risk_parity" => PositionSizingMethod::RiskParity,
            _ => PositionSizingMethod::PercentageOfCapital(Decimal::from_str_exact("0.1").unwrap()),
        };

        RiskParameters {
            max_position_size: self.risk.max_position_size,
            max_leverage: self.risk.max_leverage,
            default_leverage: self.trading.leverage,
            position_sizing_method: position_sizing,
            max_loss_per_trade: self.risk.max_loss_per_trade,
            max_daily_loss: self.risk.max_daily_loss,
        }
    }
}

// Configuration for ASTER/USDT:USDT perpetual trading
pub fn create_aster_config() -> StrategyConfig {
    StrategyConfig {
        exchanges: vec![
            ExchangeConfig {
                exchange_id: "binance_futures".to_string(),
                api_key: None,
                api_secret: None,
                test_mode: true,
            },
            ExchangeConfig {
                exchange_id: "okx".to_string(),
                api_key: None,
                api_secret: None,
                test_mode: true,
            },
        ],
        trading: TradingConfig {
            symbols: vec!["ASTER-USDT-SWAP".to_string()], // OKX format
            leverage: 10,
            order_type: "market".to_string(),
            time_in_force: "GTC".to_string(),
            enable_stop_loss: true,
            enable_take_profit: true,
            stop_loss_pct: Decimal::from_str_exact("0.03").unwrap(),  // 3% stop loss
            take_profit_pct: Decimal::from_str_exact("0.06").unwrap(), // 6% take profit
        },
        risk: RiskConfig {
            max_position_size: Decimal::from(50000), // Max $50k position
            max_leverage: 20,
            position_sizing_method: "kelly".to_string(),
            sizing_parameter: Decimal::from_str_exact("0.25").unwrap(), // 25% Kelly fraction
            max_loss_per_trade: Decimal::from_str_exact("0.03").unwrap(),
            max_daily_loss: Decimal::from_str_exact("0.10").unwrap(),
            confidence_threshold: 0.65,
            risk_threshold: 0.75,
        },
        model: ModelConfig {
            model_name: "mistral-7b-instruct".to_string(),
            model_path: Some("./models/mistral-7b".to_string()),
            use_gpu: true,
            batch_size: 4,
            prediction_threshold: 0.7,
        },
        queue: QueueConfig {
            enabled: true,
            fluvio_url: Some("localhost:9003".to_string()),
            topics: vec![
                "aster-market-data".to_string(),
                "aster-signals".to_string(),
                "aster-decisions".to_string(),
                "aster-executions".to_string(),
            ],
            consumer_group: "aster-strategy".to_string(),
        },
        backtest: BacktestConfig {
            enabled: true,
            start_date: "2024-01-01T00:00:00Z".to_string(),
            end_date: "2024-12-31T23:59:59Z".to_string(),
            initial_capital: Decimal::from(100000),
            data_source: "aster_historical.csv".to_string(),
            export_results: true,
            results_path: "aster_backtest_results.csv".to_string(),
        },
    }
}