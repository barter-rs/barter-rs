use crate::{exchange::bitmex::Bitmex, impl_market_identifier};
use barter_instrument::asset::name::AssetNameInternal;
use serde::{Deserialize, Serialize};
use smol_str::{SmolStr, StrExt, format_smolstr};

/// Type that defines how to translate a Barter [`Subscription`] into a [`Bitmex`]
/// market that can be subscribed to.
///
/// See docs: <https://www.bitmex.com/app/wsAPI>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BitmexMarket(pub SmolStr);

impl_market_identifier!(Bitmex => BitmexMarket, bitmex_market);

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
