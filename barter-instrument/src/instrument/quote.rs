use serde::{Deserialize, Serialize};

/// Instrument quote asset.
///
/// Note that all `Spot` instruments are quoted in the (underlying) quote asset.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum InstrumentQuoteAsset {
    /// "In-kind" pricing (unusual) using the underlying base asset as the quote asset.
    ///
    /// For example, if some derivative for underlying=btc_usdt was quoted in btc.
    #[serde(alias = "underlying_base")]
    UnderlyingBase,

    /// Standard pricing using the underlying quote as the quote asset.
    ///
    /// For example, all spot instruments are quoted in the (underlying) quote asset.
    #[serde(alias = "underlying_quote")]
    UnderlyingQuote,
}
