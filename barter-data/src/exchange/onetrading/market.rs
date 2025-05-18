use crate::{
    Identifier, exchange::onetrading::OneTrading, instrument::MarketInstrumentData,
    subscription::Subscription,
};
use barter_instrument::{
    Keyed, asset::name::AssetNameInternal, instrument::market_data::MarketDataInstrument,
};
use serde::{Deserialize, Serialize};
use smol_str::{SmolStr, StrExt, format_smolstr};

/// Type that defines how to translate a Barter [`Subscription`] into a [`OneTrading`]
/// market that can be subscribed to.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OneTradingMarket(pub SmolStr);

impl<Kind> Identifier<OneTradingMarket> for Subscription<OneTrading, MarketDataInstrument, Kind> {
    fn id(&self) -> OneTradingMarket {
        onetrading_market(&self.instrument.base, &self.instrument.quote)
    }
}

impl<InstrumentKey, Kind> Identifier<OneTradingMarket>
    for Subscription<OneTrading, Keyed<InstrumentKey, MarketDataInstrument>, Kind>
{
    fn id(&self) -> OneTradingMarket {
        onetrading_market(&self.instrument.value.base, &self.instrument.value.quote)
    }
}

impl<InstrumentKey, Kind> Identifier<OneTradingMarket>
    for Subscription<OneTrading, MarketInstrumentData<InstrumentKey>, Kind>
{
    fn id(&self) -> OneTradingMarket {
        OneTradingMarket(self.instrument.name_exchange.name().clone())
    }
}

impl AsRef<str> for OneTradingMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn onetrading_market(base: &AssetNameInternal, quote: &AssetNameInternal) -> OneTradingMarket {
    // Format BTC_EUR according to OneTrading format
    OneTradingMarket(format_smolstr!("{}_{}", base, quote).to_uppercase_smolstr())
}
