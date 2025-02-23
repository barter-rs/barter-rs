use crate::{
    Identifier, exchange::bitmex::Bitmex, instrument::MarketInstrumentData,
    subscription::Subscription,
};
use barter_instrument::{
    Keyed, asset::name::AssetNameInternal, instrument::market_data::MarketDataInstrument,
};
use serde::{Deserialize, Serialize};
use smol_str::{SmolStr, StrExt, format_smolstr};

/// Type that defines how to translate a Barter [`Subscription`] into a [`Bitmex`]
/// market that can be subscribed to.
///
/// See docs: <https://www.bitmex.com/app/wsAPI>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BitmexMarket(pub SmolStr);

impl<Kind> Identifier<BitmexMarket> for Subscription<Bitmex, MarketDataInstrument, Kind> {
    fn id(&self) -> BitmexMarket {
        bitmex_market(&self.instrument.base, &self.instrument.quote)
    }
}

impl<InstrumentKey, Kind> Identifier<BitmexMarket>
    for Subscription<Bitmex, Keyed<InstrumentKey, MarketDataInstrument>, Kind>
{
    fn id(&self) -> BitmexMarket {
        bitmex_market(&self.instrument.value.base, &self.instrument.value.quote)
    }
}

impl<InstrumentKey, Kind> Identifier<BitmexMarket>
    for Subscription<Bitmex, MarketInstrumentData<InstrumentKey>, Kind>
{
    fn id(&self) -> BitmexMarket {
        BitmexMarket(self.instrument.name_exchange.name().clone())
    }
}

impl AsRef<str> for BitmexMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn bitmex_market(base: &AssetNameInternal, quote: &AssetNameInternal) -> BitmexMarket {
    // Notes:
    // - Must be uppercase since Bitmex sends message with uppercase MARKET (eg/ XBTUSD).
    BitmexMarket(format_smolstr!("{base}{quote}").to_uppercase_smolstr())
}
