use barter_instrument::{
    instrument::market_data::{MarketDataInstrument, kind::MarketDataInstrumentKind},
};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct KrakenFuturesMarket(pub String);

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

impl From<MarketDataInstrument> for KrakenFuturesMarket {
    fn from(instrument: MarketDataInstrument) -> Self {
        use MarketDataInstrumentKind::*;

        // Kraken Futures symbols are uppercase.
        // BTC is XBT.
        let base = instrument.base.as_ref().to_uppercase();
        let quote = instrument.quote.as_ref().to_uppercase();
        
        let base = if base == "BTC" { "XBT".to_string() } else { base };
        let quote = if quote == "BTC" { "XBT".to_string() } else { quote };
        
        match instrument.kind {
            Perpetual => Self(format!("PI_{}{}", base, quote)),
            Future(future) => {
                let date = future.expiry.format("%y%m%d").to_string();
                Self(format!("FI_{}{}_{}", base, quote, date))
            },
            other => panic!("Kraken Futures does not support instrument kind: {}", other),
        }
    }
}
