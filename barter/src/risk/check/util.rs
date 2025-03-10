use barter_instrument::Side;
use rust_decimal::Decimal;

/// Calculates the notional value in quote asset given the quantity, price and contract_size.
///
/// The notional value represents the total value of a position.
///
/// Returns None if overflow has occurred.
///
/// # Arguments
/// * `quantity` - The number of contracts or units
/// * `price` - The price per contract/unit
///   - For standard instruments, this is typically the current market price
///   - For option instruments, this should be the strike price
/// * `contract_size` - Multiplier that determines the actual exposure per contract
pub fn calculate_quote_notional(
    quantity: Decimal,
    price: Decimal,
    contract_size: Decimal,
) -> Option<Decimal> {
    quantity.checked_mul(price)?.checked_mul(contract_size)
}

/// Calculates the absolute percentage difference between two values (eg/ prices).
///
/// Returns a `Decimal` that represents the percentage (eg/ 0.05 for a 5% difference). Will be
/// None if overflow has occurred.
pub fn calculate_abs_percent_difference(current: Decimal, other: Decimal) -> Option<Decimal> {
    // Absolute difference
    let price_diff = current.checked_sub(other)?.abs();

    // Calculate percentage difference relative to other
    price_diff.checked_div(other)
}

/// Calculate the total delta for some quantity of "in kind" units.
///
/// Delta is a measure of how an instruments price changes relative to the underlying asset.
///
/// A positive return value indicates long exposure to the underlying, while negative values
/// indicate short exposure.
///
/// # Arguments
/// * `instrument_delta` - Delta of the instrument
///   - For Spot, Perpetual, and Futures, this is 1.0.
///   - For Options, this is between -1.0 and 1.0.
/// * `contract_size` - Multiplier that determines the actual exposure per contract
/// * `side` - Quantity direction, `Side::Buy` (LONG) or `Side::Sell` (SHORT)
/// * `quantity_in_kind` - In kind quantity
pub fn calculate_delta(
    instrument_delta: Decimal,
    contract_size: Decimal,
    side: Side,
    quantity_in_kind: Decimal,
) -> Decimal {
    let delta = instrument_delta * (quantity_in_kind * contract_size);

    match side {
        Side::Buy => delta,
        Side::Sell => -delta,
    }
}
