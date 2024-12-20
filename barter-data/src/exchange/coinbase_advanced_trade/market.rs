use super::CoinbaseInternational;
use crate::{
    subscription::Subscription,
    Identifier,
};
use serde::Serialize;
use smol_str::{format_smolstr, SmolStr, StrExt};
use barter_instrument::asset::name::AssetNameInternal;
use barter_instrument::instrument::market_data::MarketDataInstrument;
use barter_instrument::Keyed;
use crate::instrument::MarketInstrumentData;

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`CoinbaseInternational`] market to be subscribed to.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct CoinbaseInternationalMarket(pub SmolStr);

impl<Kind> Identifier<CoinbaseInternationalMarket> for Subscription<CoinbaseInternational, MarketDataInstrument, Kind> {
    fn id(&self) -> CoinbaseInternationalMarket {
        coinbase_market(&self.instrument.base, &self.instrument.quote)
    }
}

impl<InstrumentKey, Kind> Identifier<CoinbaseInternationalMarket>
for Subscription<CoinbaseInternational, Keyed<InstrumentKey, MarketDataInstrument>, Kind>
{
    fn id(&self) -> CoinbaseInternationalMarket {
        coinbase_market(&self.instrument.value.base, &self.instrument.value.quote)
    }
}

impl<Kind> Identifier<CoinbaseInternationalMarket> for Subscription<CoinbaseInternational, MarketInstrumentData, Kind> {
    fn id(&self) -> CoinbaseInternationalMarket {
        CoinbaseInternationalMarket(self.instrument.name_exchange.clone())
    }
}

impl AsRef<str> for CoinbaseInternationalMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn coinbase_market(base: &AssetNameInternal, quote: &AssetNameInternal) -> CoinbaseInternationalMarket {
    CoinbaseInternationalMarket(format_smolstr!("{base}-{quote}").to_uppercase_smolstr())
}
