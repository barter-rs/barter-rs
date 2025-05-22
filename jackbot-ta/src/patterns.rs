use rust_decimal::Decimal;

/// Type of crossover event detected between two data series.
#[derive(Debug, PartialEq, Eq)]
pub enum Cross {
    Above,
    Below,
}

/// Determine if a crossover occurred between the previous and current values.
pub fn crossover(prev_fast: Decimal, prev_slow: Decimal, fast: Decimal, slow: Decimal) -> Option<Cross> {
    if prev_fast <= prev_slow && fast > slow {
        Some(Cross::Above)
    } else if prev_fast >= prev_slow && fast < slow {
        Some(Cross::Below)
    } else {
        None
    }
}
