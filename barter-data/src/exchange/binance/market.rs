use super::Binance;
use crate::impl_market_identifier;
use barter_instrument::asset::name::AssetNameInternal;
use serde::{Deserialize, Serialize};
use smol_str::{SmolStr, StrExt, format_smolstr};

/// Type that defines how to translate a Barter [`Subscription`] into a [`Binance`]
/// market that can be subscribed to.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#websocket-market-streams>
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#websocket-market-streams>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BinanceMarket(pub SmolStr);

impl_market_identifier!(Binance<Server> => BinanceMarket, binance_market);

impl AsRef<str> for BinanceMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

pub(in crate::exchange::binance) fn binance_market(
    base: &AssetNameInternal,
    quote: &AssetNameInternal,
) -> BinanceMarket {
    // Notes:
    // - Must be lowercase when subscribing (transformed to lowercase by Binance fn requests).
    // - Must be uppercase since Binance sends message with uppercase MARKET (eg/ BTCUSDT).
    BinanceMarket(format_smolstr!("{base}{quote}").to_uppercase_smolstr())
}
