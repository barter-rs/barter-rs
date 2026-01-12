use barter_integration::model::SubscriptionKind;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KrakenFuturesChannel {
    Ticker,
    Trade,
    Book,
}

impl AsRef<str> for KrakenFuturesChannel {
    fn as_ref(&self) -> &str {
        match self {
            Self::Ticker => "ticker",
            Self::Trade => "trade",
            Self::Book => "book",
        }
    }
}

impl From<SubscriptionKind> for KrakenFuturesChannel {
    fn from(kind: SubscriptionKind) -> Self {
        match kind {
            SubscriptionKind::PublicTrades | SubscriptionKind::Liquidations => Self::Trade,
            SubscriptionKind::OrderBooksL1 => Self::Ticker,
            SubscriptionKind::OrderBooksL2 => Self::Book,
            _ => panic!("KrakenFuturesChannel does not support: {:?}", kind),
        }
    }
}
