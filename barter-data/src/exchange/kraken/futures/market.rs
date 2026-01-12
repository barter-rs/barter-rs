use super::KrakenFuturesUsd;
use crate::impl_market_identifier_for_instrument;
use barter_instrument::instrument::market_data::{MarketDataInstrument, kind::MarketDataInstrumentKind};
use serde::{Deserialize, Serialize};
use smol_str::{SmolStr, format_smolstr};
use std::fmt::{Display, Formatter};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct KrakenFuturesMarket(pub SmolStr);

impl_market_identifier_for_instrument!(KrakenFuturesUsd => KrakenFuturesMarket, kraken_futures_market);

fn kraken_futures_market(instrument: &MarketDataInstrument) -> KrakenFuturesMarket {
    use MarketDataInstrumentKind::*;

    // Kraken Futures symbols are uppercase.
    // BTC is XBT on Kraken.
    let base = instrument.base.as_ref().to_uppercase();
    let quote = instrument.quote.as_ref().to_uppercase();
    
    let base = if base == "BTC" { "XBT".to_string() } else { base };
    let quote = if quote == "BTC" { "XBT".to_string() } else { quote };
    
    match &instrument.kind {
        Perpetual => KrakenFuturesMarket(format_smolstr!("PI_{}{}", base, quote)),
        Future(future) => {
            let date = future.expiry.format("%y%m%d").to_string();
            KrakenFuturesMarket(format_smolstr!("FI_{}{}_{}", base, quote, date))
        },
        other => panic!("Kraken Futures does not support instrument kind: {}", other),
    }
}

impl AsRef<str> for KrakenFuturesMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for KrakenFuturesMarket {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kraken_futures_market_as_ref() {
        let market = KrakenFuturesMarket(SmolStr::new("PI_XBTUSD"));
        assert_eq!(market.as_ref(), "PI_XBTUSD");
    }

    #[test]
    fn test_kraken_futures_market_display() {
        let market = KrakenFuturesMarket(SmolStr::new("PI_XBTUSD"));
        assert_eq!(format!("{}", market), "PI_XBTUSD");
    }

    #[test]
    fn test_kraken_futures_market_perpetual_format() {
        // Perpetuals should use PI_ prefix
        let market = KrakenFuturesMarket(SmolStr::new("PI_XBTUSD"));
        assert!(market.0.starts_with("PI_"));
    }

    #[test]
    fn test_kraken_futures_market_future_format() {
        // Fixed-maturity futures should use FI_ prefix with date
        let market = KrakenFuturesMarket(SmolStr::new("FI_XBTUSD_240315"));
        assert!(market.0.starts_with("FI_"));
        assert!(market.0.contains("_240315"));
    }

    #[test]
    fn test_kraken_futures_market_serde_roundtrip() {
        let market = KrakenFuturesMarket(SmolStr::new("PI_ETHUSD"));
        let serialized = serde_json::to_string(&market).unwrap();
        let deserialized: KrakenFuturesMarket = serde_json::from_str(&serialized).unwrap();
        assert_eq!(market, deserialized);
    }

    #[test]
    fn test_kraken_futures_market_equality() {
        let market1 = KrakenFuturesMarket(SmolStr::new("PI_XBTUSD"));
        let market2 = KrakenFuturesMarket(SmolStr::new("PI_XBTUSD"));
        let market3 = KrakenFuturesMarket(SmolStr::new("PI_ETHUSD"));
        
        assert_eq!(market1, market2);
        assert_ne!(market1, market3);
    }
}