use rust_decimal::Decimal;
use std::collections::VecDeque;

/// Simple moving average indicator.
#[derive(Debug, Clone)]
pub struct SimpleMovingAverage {
    period: usize,
    values: VecDeque<Decimal>,
    sum: Decimal,
}

impl SimpleMovingAverage {
    /// Create a new SMA with the given period.
    pub fn new(period: usize) -> Self {
        Self { period, values: VecDeque::new(), sum: Decimal::ZERO }
    }

    /// Update the SMA with a new value and return the latest average.
    pub fn update(&mut self, value: Decimal) -> Decimal {
        self.values.push_back(value);
        self.sum += value;
        if self.values.len() > self.period {
            if let Some(old) = self.values.pop_front() {
                self.sum -= old;
            }
        }
        self.average()
    }

    /// Current average value.
    pub fn average(&self) -> Decimal {
        if self.values.is_empty() {
            Decimal::ZERO
        } else {
            self.sum / Decimal::from(self.values.len() as u64)
        }
    }
}

/// Exponential moving average indicator.
#[derive(Debug, Clone)]
pub struct ExponentialMovingAverage {
    multiplier: Decimal,
    value: Option<Decimal>,
}

impl ExponentialMovingAverage {
    /// Create a new EMA with the given period.
    pub fn new(period: usize) -> Self {
        let multiplier = Decimal::from(2u64) / Decimal::from(period as u64 + 1);
        Self { multiplier, value: None }
    }

    /// Update the EMA with a new price and return the latest value.
    pub fn update(&mut self, price: Decimal) -> Decimal {
        match self.value {
            Some(val) => {
                let next = (price - val) * self.multiplier + val;
                self.value = Some(next);
                next
            }
            None => {
                self.value = Some(price);
                price
            }
        }
    }

    /// Current EMA value if initialised.
    pub fn value(&self) -> Option<Decimal> {
        self.value
    }
}
