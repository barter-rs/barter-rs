use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use derive_more::Constructor;
use chrono::{DateTime, Duration, Utc};

/// Quote prices maintained by the market making logic.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, Constructor)]
pub struct Quote {
    /// Bid price to place on the exchange.
    pub bid_price: Decimal,
    /// Ask price to place on the exchange.
    pub ask_price: Decimal,
}

/// Direction of a trade used for flow analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum TradeSide {
    Buy,
    Sell,
}

/// Detector for order flow toxicity based on directional volume ratios.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, Constructor)]
pub struct FlowToxicityDetector {
    /// Minimum ratio of one-sided volume to total volume considered toxic.
    pub threshold: Decimal,
}

impl FlowToxicityDetector {
    /// Determine if the provided trades exhibit toxic flow.
    /// Each trade is a tuple of (`TradeSide`, `volume`).
    pub fn is_toxic(&self, trades: &[(TradeSide, Decimal)]) -> bool {
        let (buy, sell) = trades.iter().fold((Decimal::ZERO, Decimal::ZERO), |acc, (side, vol)| match side {
            TradeSide::Buy => (acc.0 + *vol, acc.1),
            TradeSide::Sell => (acc.0, acc.1 + *vol),
        });
        let total = buy + sell;
        if total == Decimal::ZERO {
            return false;
        }
        let dominant = if buy > sell { buy } else { sell };
        dominant / total > self.threshold
    }
}

/// Helper for tracking when quotes should be refreshed.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, Constructor)]
pub struct QuoteRefresher {
    /// Interval at which quotes should be refreshed.
    pub refresh_interval: Duration,
    #[serde(skip)]
    last_refresh: Option<DateTime<Utc>>,
}

impl QuoteRefresher {
    /// Record the time of the most recent quote refresh.
    pub fn record_refresh(&mut self, time: DateTime<Utc>) {
        self.last_refresh = Some(time);
    }

    /// Determine if quotes should be refreshed given `now`.
    pub fn needs_refresh(&self, now: DateTime<Utc>) -> bool {
        match self.last_refresh {
            Some(last) => now - last >= self.refresh_interval,
            None => true,
        }
    }
}

/// Reactively adjust quotes based on recent flow direction.
pub fn reactive_adjust(mut quote: Quote, side: TradeSide, amount: Decimal) -> Quote {
    match side {
        TradeSide::Buy => {
            quote.bid_price += amount;
            quote.ask_price += amount;
        }
        TradeSide::Sell => {
            quote.bid_price -= amount;
            quote.ask_price -= amount;
        }
    }
    quote
}

/// Predictively adjust quotes around a forecasted mid price while
/// maintaining the existing spread.
pub fn predictive_adjust(current: Quote, predicted_mid: Decimal) -> Quote {
    let spread = current.ask_price - current.bid_price;
    let half = spread / Decimal::from(2);
    Quote::new(predicted_mid - half, predicted_mid + half)
}

