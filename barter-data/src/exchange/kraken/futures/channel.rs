use super::KrakenFuturesUsd;
use crate::{
    impl_channel_identifier,
    subscription::{
        SubKind,
        book::{OrderBooksL1, OrderBooksL2},
        liquidation::Liquidations,
        trade::PublicTrades,
    },
};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KrakenFuturesChannel {
    Ticker,
    Trade,
    Book,
}

impl_channel_identifier!(KrakenFuturesUsd, Instrument => KrakenFuturesChannel, PublicTrades => KrakenFuturesChannel::Trade);
impl_channel_identifier!(KrakenFuturesUsd, Instrument => KrakenFuturesChannel, Liquidations => KrakenFuturesChannel::Trade);
impl_channel_identifier!(KrakenFuturesUsd, Instrument => KrakenFuturesChannel, OrderBooksL1 => KrakenFuturesChannel::Ticker);
impl_channel_identifier!(KrakenFuturesUsd, Instrument => KrakenFuturesChannel, OrderBooksL2 => KrakenFuturesChannel::Book);

impl AsRef<str> for KrakenFuturesChannel {
    fn as_ref(&self) -> &str {
        match self {
            Self::Ticker => "ticker",
            Self::Trade => "trade",
            Self::Book => "book",
        }
    }
}

impl From<SubKind> for KrakenFuturesChannel {
    fn from(kind: SubKind) -> Self {
        match kind {
            SubKind::PublicTrades | SubKind::Liquidations => Self::Trade,
            SubKind::OrderBooksL1 => Self::Ticker,
            SubKind::OrderBooksL2 => Self::Book,
            _ => panic!("KrakenFuturesChannel does not support: {:?}", kind),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kraken_futures_channel_ticker() {
        let channel = KrakenFuturesChannel::Ticker;
        assert_eq!(channel.as_ref(), "ticker");
    }

    #[test]
    fn test_kraken_futures_channel_trade() {
        let channel = KrakenFuturesChannel::Trade;
        assert_eq!(channel.as_ref(), "trade");
    }

    #[test]
    fn test_kraken_futures_channel_book() {
        let channel = KrakenFuturesChannel::Book;
        assert_eq!(channel.as_ref(), "book");
    }

    #[test]
    fn test_kraken_futures_channel_from_public_trades() {
        let channel = KrakenFuturesChannel::from(SubKind::PublicTrades);
        assert_eq!(channel, KrakenFuturesChannel::Trade);
    }

    #[test]
    fn test_kraken_futures_channel_from_liquidations() {
        let channel = KrakenFuturesChannel::from(SubKind::Liquidations);
        assert_eq!(channel, KrakenFuturesChannel::Trade);
    }

    #[test]
    fn test_kraken_futures_channel_from_order_books_l1() {
        let channel = KrakenFuturesChannel::from(SubKind::OrderBooksL1);
        assert_eq!(channel, KrakenFuturesChannel::Ticker);
    }

    #[test]
    fn test_kraken_futures_channel_from_order_books_l2() {
        let channel = KrakenFuturesChannel::from(SubKind::OrderBooksL2);
        assert_eq!(channel, KrakenFuturesChannel::Book);
    }

    #[test]
    fn test_kraken_futures_channel_serde() {
        let channel = KrakenFuturesChannel::Trade;
        let serialized = serde_json::to_string(&channel).unwrap();
        assert_eq!(serialized, r#""trade""#);
    }
}