use crate::{exchange::bybit::Bybit, impl_market_identifier};
use barter_instrument::asset::name::AssetNameInternal;
use serde::{Deserialize, Serialize};
use smol_str::{SmolStr, StrExt, format_smolstr};

/// Type that defines how to translate a Barter [`Subscription`] into a [`Bybit`]
/// market that can be subscribed to.
///
/// See docs: <https://bybit-exchange.github.io/docs/v5/ws/connect>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BybitMarket(pub SmolStr);

impl_market_identifier!(Bybit<Server> => BybitMarket, bybit_market);

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
