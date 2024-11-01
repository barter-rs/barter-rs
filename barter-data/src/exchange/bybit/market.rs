use crate::{
    exchange::bybit::Bybit, instrument::MarketInstrumentData, subscription::Subscription,
    Identifier,
};
use barter_instrument::{
    asset::name::AssetNameInternal, instrument::market_data::MarketDataInstrument, Keyed,
};
use serde::{Deserialize, Serialize};
use smol_str::{format_smolstr, SmolStr, StrExt};

/// Type that defines how to translate a Barter [`Subscription`] into a [`Bybit`]
/// market that can be subscribed to.
///
/// See docs: <https://bybit-exchange.github.io/docs/v5/ws/connect>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BybitMarket(pub SmolStr);

impl<Server, Kind> Identifier<BybitMarket>
    for Subscription<Bybit<Server>, MarketDataInstrument, Kind>
{
    fn id(&self) -> BybitMarket {
        bybit_market(&self.instrument.base, &self.instrument.quote)
    }
}

impl<Server, InstrumentKey, Kind> Identifier<BybitMarket>
    for Subscription<Bybit<Server>, Keyed<InstrumentKey, MarketDataInstrument>, Kind>
{
    fn id(&self) -> BybitMarket {
        bybit_market(&self.instrument.value.base, &self.instrument.value.quote)
    }
}

impl<Server, Kind> Identifier<BybitMarket>
    for Subscription<Bybit<Server>, MarketInstrumentData, Kind>
{
    fn id(&self) -> BybitMarket {
        BybitMarket(self.instrument.name_exchange.clone())
    }
}

impl AsRef<str> for BybitMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn bybit_market(base: &AssetNameInternal, quote: &AssetNameInternal) -> BybitMarket {
    // Notes:
    // - Must be uppercase since Bybit sends message with uppercase MARKET (eg/ BTCUSDT).
    BybitMarket(format_smolstr!("{base}{quote}").to_uppercase_smolstr())
}
