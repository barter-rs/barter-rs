use derive_more::{Constructor, Display};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct ExchangeIndex(pub usize);

impl ExchangeIndex {
    pub fn index(&self) -> usize {
        self.0
    }
}

impl std::fmt::Display for ExchangeIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ExchangeIndex({})", self.0)
    }
}

/// Unique identifier for an execution server.
///
/// ### Notes
/// An execution may have a distinct server for different
/// [`InstrumentKinds`](super::instrument::kind::InstrumentKind).
///
/// For example, BinanceSpot and BinanceFuturesUsd have distinct APIs, and are therefore
/// represented as unique variants.
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display,
)]
#[serde(rename = "execution", rename_all = "snake_case")]
pub enum ExchangeId {
    Other,
    Simulated,
    Mock,
    BinanceFuturesCoin,
    BinanceFuturesUsd,
    BinanceOptions,
    BinancePortfolioMargin,
    BinanceSpot,
    BinanceUs,
    Bitazza,
    Bitfinex,
    Bitflyer,
    Bitget,
    Bitmart,
    BitmartFuturesUsd,
    Bitmex,
    Bitso,
    Bitstamp,
    Bitvavo,
    Bithumb,
    BybitPerpetualsUsd,
    BybitSpot,
    Cexio,
    Coinbase,
    CoinbaseInternational,
    Cryptocom,
    Deribit,
    GateioFuturesBtc,
    GateioFuturesUsd,
    GateioOptions,
    GateioPerpetualsBtc,
    GateioPerpetualsUsd,
    GateioSpot,
    Gemini,
    Hitbtc,
    #[serde(alias = "huobi")]
    Htx,
    Kraken,
    Kucoin,
    Liquid,
    Mexc,
    Okx,
    Poloniex,
}

impl ExchangeId {
    /// Return the &str representation of this [`ExchangeId`]
    pub fn as_str(&self) -> &'static str {
        match self {
            ExchangeId::Other => "other",
            ExchangeId::Simulated => "simulated",
            ExchangeId::Mock => "mock",
            ExchangeId::BinanceFuturesCoin => "binance_futures_coin",
            ExchangeId::BinanceFuturesUsd => "binance_futures_usd",
            ExchangeId::BinanceOptions => "binance_options",
            ExchangeId::BinancePortfolioMargin => "binance_portfolio_margin",
            ExchangeId::BinanceSpot => "binance_spot",
            ExchangeId::BinanceUs => "binance_us",
            ExchangeId::Bitazza => "bitazza",
            ExchangeId::Bitfinex => "bitfinex",
            ExchangeId::Bitflyer => "bitflyer",
            ExchangeId::Bitget => "bitget",
            ExchangeId::Bitmart => "bitmart",
            ExchangeId::BitmartFuturesUsd => "bitmart_futures_usd",
            ExchangeId::Bitmex => "bitmex",
            ExchangeId::Bitso => "bitso",
            ExchangeId::Bitstamp => "bitstamp",
            ExchangeId::Bitvavo => "bitvavo",
            ExchangeId::Bithumb => "bithumb",
            ExchangeId::BybitPerpetualsUsd => "bybit_perpetuals_usd",
            ExchangeId::BybitSpot => "bybit_spot",
            ExchangeId::Cexio => "cexio",
            ExchangeId::Coinbase => "coinbase",
            ExchangeId::CoinbaseInternational => "coinbase_international",
            ExchangeId::Cryptocom => "cryptocom",
            ExchangeId::Deribit => "deribit",
            ExchangeId::GateioFuturesBtc => "gateio_futures_btc",
            ExchangeId::GateioFuturesUsd => "gateio_futures_usd",
            ExchangeId::GateioOptions => "gateio_options",
            ExchangeId::GateioPerpetualsBtc => "gateio_perpetuals_btc",
            ExchangeId::GateioPerpetualsUsd => "gateio_perpetuals_usd",
            ExchangeId::GateioSpot => "gateio_spot",
            ExchangeId::Gemini => "gemini",
            ExchangeId::Hitbtc => "hitbtc",
            ExchangeId::Htx => "htx", // huobi alias
            ExchangeId::Kraken => "kraken",
            ExchangeId::Kucoin => "kucoin",
            ExchangeId::Liquid => "liquid",
            ExchangeId::Mexc => "mexc",
            ExchangeId::Okx => "okx",
            ExchangeId::Poloniex => "poloniex",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_de_exchange_id() {
        assert_eq!(
            serde_json::from_str::<ExchangeId>(r#""htx""#).unwrap(),
            ExchangeId::Htx
        );
        assert_eq!(
            serde_json::from_str::<ExchangeId>(r#""huobi""#).unwrap(),
            ExchangeId::Htx
        );
    }
}
