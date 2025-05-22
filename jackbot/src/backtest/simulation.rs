use rust_decimal::Decimal;
use std::time::Duration;

/// Configuration parameters for [`MarketSimulator`].
#[derive(Debug, Clone, Copy)]
pub struct SimulationConfig {
    /// Round trip latency applied to each order.
    pub latency: Duration,
    /// Slippage in basis points applied to executed orders.
    pub slippage_bps: f64,
    /// Fees in basis points charged per trade.
    pub fee_bps: f64,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            latency: Duration::from_millis(0),
            slippage_bps: 0.0,
            fee_bps: 0.0,
        }
    }
}

/// Result of a simulated trade execution.
#[derive(Debug, Clone)]
pub struct TradeResult {
    pub executed_price: Decimal,
    pub fee: Decimal,
}

/// Simple market simulator applying latency, slippage and fees to orders.
#[derive(Debug, Clone)]
pub struct MarketSimulator {
    config: SimulationConfig,
}

impl MarketSimulator {
    /// Create a new simulator from the given configuration.
    pub fn new(config: SimulationConfig) -> Self {
        Self { config }
    }

    /// Simulate execution of an order at the given price and quantity.
    pub fn execute(&self, price: Decimal, quantity: Decimal) -> TradeResult {
        let slip = price * Decimal::from_f64(self.config.slippage_bps / 10_000.0).unwrap_or_default();
        let executed_price = price + slip;
        let fee = executed_price * quantity * Decimal::from_f64(self.config.fee_bps / 10_000.0).unwrap_or_default();
        TradeResult { executed_price, fee }
    }

    /// Return the configured latency.
    pub fn latency(&self) -> Duration {
        self.config.latency
    }
}
