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
///
/// OneTrading uses a format where the base and quote currencies are joined with an underscore
/// and uppercase: `BASE_QUOTE`.
///
/// # Examples
///
/// ```
/// use barter_data::exchange::onetrading::market::OneTradingMarket;
/// use smol_str::SmolStr;
///
/// // Directly create an OneTrading market
/// let btc_eur = OneTradingMarket(SmolStr::new("BTC_EUR"));
///
/// // Using the from_base_quote_internal method with AssetNameInternal
/// use barter_instrument::asset::name::AssetNameInternal;
/// let base = AssetNameInternal::from("BTC");
/// let quote = AssetNameInternal::from("EUR");
///
/// let market = OneTradingMarket::from_base_quote_internal(&base, &quote);
/// assert_eq!(market.as_ref(), "BTC_EUR");
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OneTradingMarket(pub SmolStr);

impl OneTradingMarket {
    /// Creates a new [`OneTradingMarket`] from base and quote asset names.
    ///
    /// Formats the market symbol according to OneTrading's convention: `BASE_QUOTE`
    ///
    /// # Examples
    ///
    /// ```
    /// use barter_instrument::asset::name::AssetNameInternal;
    /// use barter_data::exchange::onetrading::market::OneTradingMarket;
    ///
    /// let base = AssetNameInternal::from("btc");
    /// let quote = AssetNameInternal::from("eur");
    ///
    /// let market = OneTradingMarket::from_base_quote_internal(&base, &quote);
    /// assert_eq!(market.as_ref(), "BTC_EUR"); // Note: converted to uppercase
    /// ```
    pub fn from_base_quote_internal(base: &AssetNameInternal, quote: &AssetNameInternal) -> Self {
        // Format BTC_EUR according to OneTrading format
        Self(format_smolstr!("{}_{}", base, quote).to_uppercase_smolstr())
    }
}

impl<Kind> Identifier<OneTradingMarket> for Subscription<OneTrading, MarketDataInstrument, Kind> {
    fn id(&self) -> OneTradingMarket {
        OneTradingMarket::from_base_quote_internal(&self.instrument.base, &self.instrument.quote)
    }
}

impl<InstrumentKey, Kind> Identifier<OneTradingMarket>
    for Subscription<OneTrading, Keyed<InstrumentKey, MarketDataInstrument>, Kind>
{
    fn id(&self) -> OneTradingMarket {
        OneTradingMarket::from_base_quote_internal(
            &self.instrument.value.base,
            &self.instrument.value.quote,
        )
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
