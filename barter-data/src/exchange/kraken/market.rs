use super::KrakenSpot;
use crate::impl_market_identifier;
use barter_instrument::asset::name::AssetNameInternal;
use serde::{Deserialize, Serialize};
use smol_str::{SmolStr, StrExt, format_smolstr};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Kraken`](super::spot::KrakenSpot) market that can be subscribed to.
///
/// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct KrakenMarket(pub SmolStr);

impl_market_identifier!(KrakenSpot => KrakenMarket, kraken_market);

impl AsRef<str> for KrakenMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn kraken_market(base: &AssetNameInternal, quote: &AssetNameInternal) -> KrakenMarket {
    KrakenMarket(format_smolstr!("{base}/{quote}").to_uppercase_smolstr())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kraken_market_formatting() {
        let market = KrakenMarket(smol_str::SmolStr::new("xbt/usd"));
        assert_eq!(market.as_ref(), "xbt/usd");
    }

    #[test]
    fn test_kraken_market_from_function() {
        let base = AssetNameInternal::from("btc");
        let quote = AssetNameInternal::from("usd");
        let market = kraken_market(&base, &quote);
        // Kraken uses uppercase format with slash separator
        assert_eq!(market.as_ref(), "BTC/USD");
    }

    #[test]
    fn test_kraken_market_uppercase_normalization() {
        let base = AssetNameInternal::from("xbt");
        let quote = AssetNameInternal::from("eur");
        let market = kraken_market(&base, &quote);
        assert_eq!(market.as_ref(), "XBT/EUR");
    }

    #[test]
    fn test_kraken_market_serde_roundtrip() {
        let market = KrakenMarket(smol_str::SmolStr::new("ETH/USD"));
        let serialized = serde_json::to_string(&market).unwrap();
        let deserialized: KrakenMarket = serde_json::from_str(&serialized).unwrap();
        assert_eq!(market, deserialized);
    }
}