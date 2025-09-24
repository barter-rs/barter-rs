use crate::{
    signal::{MarketSignal, SignalData, TradeSide},
    Result, StrategyError,
};
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use ta::{indicators::*, DataItem};
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedSignal {
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub features: Features,
    pub indicators: TechnicalIndicators,
    pub market_microstructure: MarketMicrostructure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Features {
    pub price: Decimal,
    pub volume: Decimal,
    pub spread: Decimal,
    pub volatility: f64,
    pub momentum: f64,
    pub order_imbalance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalIndicators {
    pub sma_20: Option<f64>,
    pub sma_50: Option<f64>,
    pub ema_12: Option<f64>,
    pub ema_26: Option<f64>,
    pub rsi: Option<f64>,
    pub macd: Option<f64>,
    pub macd_signal: Option<f64>,
    pub bollinger_upper: Option<f64>,
    pub bollinger_lower: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketMicrostructure {
    pub bid_ask_spread: Decimal,
    pub bid_volume: Decimal,
    pub ask_volume: Decimal,
    pub trade_intensity: f64,
    pub price_impact: f64,
}

pub struct SignalProcessor {
    symbol: String,
    price_history: VecDeque<f64>,
    volume_history: VecDeque<f64>,
    trade_count: HashMap<DateTime<Utc>, u32>,
    window_size: usize,
}

impl SignalProcessor {
    pub fn new(symbol: String, window_size: usize) -> Self {
        Self {
            symbol,
            price_history: VecDeque::with_capacity(window_size),
            volume_history: VecDeque::with_capacity(window_size),
            trade_count: HashMap::new(),
            window_size,
        }
    }

    pub async fn process(&mut self, signal: MarketSignal) -> Result<ProcessedSignal> {
        info!("Processing signal for {}", signal.symbol);

        // Extract basic features
        let features = self.extract_features(&signal)?;

        // Calculate technical indicators
        let indicators = self.calculate_indicators()?;

        // Analyze market microstructure
        let market_microstructure = self.analyze_microstructure(&signal)?;

        Ok(ProcessedSignal {
            timestamp: signal.timestamp,
            symbol: signal.symbol,
            features,
            indicators,
            market_microstructure,
        })
    }

    fn extract_features(&mut self, signal: &MarketSignal) -> Result<Features> {
        let (price, volume, spread) = match &signal.data {
            SignalData::Trade { price, amount, .. } => {
                let price_f64 = price.to_string().parse::<f64>().unwrap_or(0.0);
                let volume_f64 = amount.to_string().parse::<f64>().unwrap_or(0.0);

                self.update_history(price_f64, volume_f64);

                (*price, *amount, Decimal::ZERO)
            }
            SignalData::OrderBookL1 {
                bid_price,
                ask_price,
                bid_amount,
                ask_amount,
            } => {
                let mid_price = (*bid_price + *ask_price) / Decimal::from(2);
                let total_volume = *bid_amount + *ask_amount;
                let spread = *ask_price - *bid_price;

                let price_f64 = mid_price.to_string().parse::<f64>().unwrap_or(0.0);
                let volume_f64 = total_volume.to_string().parse::<f64>().unwrap_or(0.0);

                self.update_history(price_f64, volume_f64);

                (mid_price, total_volume, spread)
            }
            _ => (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO),
        };

        let volatility = self.calculate_volatility();
        let momentum = self.calculate_momentum();
        let order_imbalance = self.calculate_order_imbalance(signal);

        Ok(Features {
            price,
            volume,
            spread,
            volatility,
            momentum,
            order_imbalance,
        })
    }

    fn update_history(&mut self, price: f64, volume: f64) {
        self.price_history.push_back(price);
        if self.price_history.len() > self.window_size {
            self.price_history.pop_front();
        }

        self.volume_history.push_back(volume);
        if self.volume_history.len() > self.window_size {
            self.volume_history.pop_front();
        }
    }

    fn calculate_indicators(&self) -> Result<TechnicalIndicators> {
        let prices: Vec<f64> = self.price_history.iter().copied().collect();

        if prices.len() < 2 {
            return Ok(TechnicalIndicators {
                sma_20: None,
                sma_50: None,
                ema_12: None,
                ema_26: None,
                rsi: None,
                macd: None,
                macd_signal: None,
                bollinger_upper: None,
                bollinger_lower: None,
            });
        }

        // Simple Moving Averages
        let sma_20 = if prices.len() >= 20 {
            Some(prices.iter().rev().take(20).sum::<f64>() / 20.0)
        } else {
            None
        };

        let sma_50 = if prices.len() >= 50 {
            Some(prices.iter().rev().take(50).sum::<f64>() / 50.0)
        } else {
            None
        };

        // Exponential Moving Averages
        let ema_12 = self.calculate_ema(&prices, 12);
        let ema_26 = self.calculate_ema(&prices, 26);

        // RSI
        let rsi = self.calculate_rsi(&prices, 14);

        // MACD
        let (macd, macd_signal) = if let (Some(e12), Some(e26)) = (ema_12, ema_26) {
            let macd_val = e12 - e26;
            (Some(macd_val), Some(macd_val * 0.8)) // Simplified signal line
        } else {
            (None, None)
        };

        // Bollinger Bands
        let (bollinger_upper, bollinger_lower) = if let Some(sma) = sma_20 {
            let std_dev = self.calculate_std_dev(&prices, 20);
            (Some(sma + 2.0 * std_dev), Some(sma - 2.0 * std_dev))
        } else {
            (None, None)
        };

        Ok(TechnicalIndicators {
            sma_20,
            sma_50,
            ema_12,
            ema_26,
            rsi,
            macd,
            macd_signal,
            bollinger_upper,
            bollinger_lower,
        })
    }

    fn calculate_ema(&self, prices: &[f64], period: usize) -> Option<f64> {
        if prices.len() < period {
            return None;
        }

        let k = 2.0 / (period as f64 + 1.0);
        let mut ema = prices[0];

        for price in prices.iter().skip(1) {
            ema = price * k + ema * (1.0 - k);
        }

        Some(ema)
    }

    fn calculate_rsi(&self, prices: &[f64], period: usize) -> Option<f64> {
        if prices.len() < period + 1 {
            return None;
        }

        let mut gains = 0.0;
        let mut losses = 0.0;

        for i in 1..=period {
            let change = prices[i] - prices[i - 1];
            if change > 0.0 {
                gains += change;
            } else {
                losses += change.abs();
            }
        }

        let avg_gain = gains / period as f64;
        let avg_loss = losses / period as f64;

        if avg_loss == 0.0 {
            return Some(100.0);
        }

        let rs = avg_gain / avg_loss;
        Some(100.0 - (100.0 / (1.0 + rs)))
    }

    fn calculate_std_dev(&self, prices: &[f64], period: usize) -> f64 {
        if prices.len() < period {
            return 0.0;
        }

        let recent: Vec<f64> = prices.iter().rev().take(period).copied().collect();
        let mean = recent.iter().sum::<f64>() / period as f64;
        let variance = recent.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / period as f64;

        variance.sqrt()
    }

    fn calculate_volatility(&self) -> f64 {
        if self.price_history.len() < 2 {
            return 0.0;
        }

        let returns: Vec<f64> = self
            .price_history
            .iter()
            .zip(self.price_history.iter().skip(1))
            .map(|(prev, curr)| (curr / prev).ln())
            .collect();

        if returns.is_empty() {
            return 0.0;
        }

        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance = returns
            .iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>()
            / returns.len() as f64;

        variance.sqrt() * (252.0_f64).sqrt() // Annualized volatility
    }

    fn calculate_momentum(&self) -> f64 {
        if self.price_history.len() < 10 {
            return 0.0;
        }

        let recent = self.price_history.back().unwrap_or(&0.0);
        let past = self.price_history.iter().rev().nth(9).unwrap_or(&0.0);

        if *past == 0.0 {
            return 0.0;
        }

        (recent - past) / past
    }

    fn calculate_order_imbalance(&self, signal: &MarketSignal) -> f64 {
        match &signal.data {
            SignalData::OrderBookL1 {
                bid_amount,
                ask_amount,
                ..
            } => {
                let bid_f64 = bid_amount.to_string().parse::<f64>().unwrap_or(0.0);
                let ask_f64 = ask_amount.to_string().parse::<f64>().unwrap_or(0.0);
                let total = bid_f64 + ask_f64;

                if total == 0.0 {
                    0.0
                } else {
                    (bid_f64 - ask_f64) / total
                }
            }
            _ => 0.0,
        }
    }

    fn analyze_microstructure(&self, signal: &MarketSignal) -> Result<MarketMicrostructure> {
        let (bid_ask_spread, bid_volume, ask_volume) = match &signal.data {
            SignalData::OrderBookL1 {
                bid_price,
                ask_price,
                bid_amount,
                ask_amount,
            } => (*ask_price - *bid_price, *bid_amount, *ask_amount),
            _ => (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO),
        };

        let trade_intensity = self.calculate_trade_intensity(signal.timestamp);
        let price_impact = self.estimate_price_impact();

        Ok(MarketMicrostructure {
            bid_ask_spread,
            bid_volume,
            ask_volume,
            trade_intensity,
            price_impact,
        })
    }

    fn calculate_trade_intensity(&self, timestamp: DateTime<Utc>) -> f64 {
        let window_start = timestamp - Duration::minutes(1);
        let count = self
            .trade_count
            .iter()
            .filter(|(ts, _)| **ts >= window_start && **ts <= timestamp)
            .map(|(_, count)| *count as f64)
            .sum::<f64>();

        count / 60.0 // Trades per second
    }

    fn estimate_price_impact(&self) -> f64 {
        // Simplified price impact estimation
        if self.price_history.len() < 2 {
            return 0.0;
        }

        let recent_price = self.price_history.back().unwrap_or(&0.0);
        let prev_price = self.price_history.iter().rev().nth(1).unwrap_or(&0.0);

        if *prev_price == 0.0 {
            0.0
        } else {
            (recent_price - prev_price).abs() / prev_price
        }
    }
}