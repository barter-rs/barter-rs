/// Defines a global [`ExchangeId`] enum covering all exchanges.
pub mod exchange;

pub mod asset;
/// [`Instrument`] related data structures.
///
/// eg/ `Instrument`, `InstrumentKind`, `OptionContract`, `Symbol`, etc.
pub mod instrument;
pub mod market;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
