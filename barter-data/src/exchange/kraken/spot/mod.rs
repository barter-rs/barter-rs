use crate::{
    NoInitialSnapshots,
    exchange::{ExchangeServer, StreamSelector, Connector, ExchangeSub},
    instrument::InstrumentData,
    subscription::{book::{OrderBooksL1, OrderBooksL2}, trade::PublicTrades},
    transformer::stateless::StatelessTransformer,
    subscriber::{WebSocketSubscriber, validator::WebSocketSubValidator},
};
use barter_integration::{error::SocketError, protocol::websocket::WsMessage};
use barter_instrument::exchange::ExchangeId;
use url::Url;
use serde_json::json;
use super::{KrakenExchange, KrakenWsStream, channel::KrakenChannel, market::KrakenMarket, subscription::KrakenSubResponse};
use self::{
    book::{l1::KrakenOrderBookL1, l2::KrakenOrderBookL2}, trade::KrakenTrades,
};

pub mod book;
pub mod trade;

/// [`KrakenSpot`] execution server.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct KrakenServerSpot;

impl ExchangeServer for KrakenServerSpot {
    const ID: ExchangeId = ExchangeId::Kraken;

    fn websocket_url() -> &'static str {
        "wss://ws.kraken.com/"
    }
}

/// Type alias for [`Kraken`](super::Kraken) Spot exchange configuration.
pub type KrakenSpot = KrakenExchange<KrakenServerSpot>;

impl Connector for KrakenSpot {
    const ID: ExchangeId = KrakenServerSpot::ID;
    type Channel = KrakenChannel;
    type Market = KrakenMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = KrakenSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(KrakenServerSpot::websocket_url()).map_err(SocketError::UrlParse)
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        exchange_subs
            .into_iter()
            .map(|ExchangeSub { channel, market }| {
                let subscription = match channel {
                    KrakenChannel::OrderBookL2 => json!({
                        "name": channel.as_ref(), // "book"
                        "depth": 100
                    }),
                    _ => json!({
                        "name": channel.as_ref()
                    }),
                };

                WsMessage::text(
                    json!({
                        "event": "subscribe",
                        "pair": [market.as_ref()],
                        "subscription": subscription
                    })
                    .to_string(),
                )
            })
            .collect()
    }
}

impl<Instrument> StreamSelector<Instrument, PublicTrades> for KrakenSpot
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream =
        KrakenWsStream<StatelessTransformer<Self, Instrument::Key, PublicTrades, KrakenTrades>>;
}

impl<Instrument> StreamSelector<Instrument, OrderBooksL1> for KrakenSpot
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = KrakenWsStream<
        StatelessTransformer<Self, Instrument::Key, OrderBooksL1, KrakenOrderBookL1>,
    >;
}

impl<Instrument> StreamSelector<Instrument, OrderBooksL2> for KrakenSpot
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = KrakenWsStream<
        StatelessTransformer<Self, Instrument::Key, OrderBooksL2, KrakenOrderBookL2>,
    >;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kraken_spot_connector_id() {
        assert_eq!(<KrakenSpot as Connector>::ID, ExchangeId::Kraken);
    }

    #[test]
    fn test_kraken_spot_websocket_url() {
        let url = KrakenServerSpot::websocket_url();
        assert_eq!(url, "wss://ws.kraken.com/");
    }

    #[test]
    fn test_kraken_spot_url_parsing() {
        let url = KrakenSpot::url().expect("Failed to parse URL");
        assert_eq!(url.scheme(), "wss");
        assert_eq!(url.host_str(), Some("ws.kraken.com"));
    }

    #[test]
    fn test_kraken_spot_requests_trades() {
        use crate::exchange::kraken::channel::KrakenChannel;
        use crate::exchange::kraken::market::KrakenMarket;
        use smol_str::SmolStr;

        let exchange_subs = vec![ExchangeSub {
            channel: KrakenChannel::Trades,
            market: KrakenMarket(SmolStr::new("XBT/USD")),
        }];

        let requests = KrakenSpot::requests(exchange_subs);
        assert_eq!(requests.len(), 1);

        let expected = serde_json::json!({
            "event": "subscribe",
            "pair": ["XBT/USD"],
            "subscription": {
                "name": "trade"
            }
        });
        
        if let WsMessage::Text(text) = &requests[0] {
            let actual: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(actual, expected);
        } else {
            panic!("Expected WsMessage::Text");
        }
    }

    #[test]
    fn test_kraken_spot_requests_book_l2_includes_depth() {
        use crate::exchange::kraken::channel::KrakenChannel;
        use crate::exchange::kraken::market::KrakenMarket;
        use smol_str::SmolStr;

        let exchange_subs = vec![ExchangeSub {
            channel: KrakenChannel::OrderBookL2,
            market: KrakenMarket(SmolStr::new("XBT/USD")),
        }];

        let requests = KrakenSpot::requests(exchange_subs);
        assert_eq!(requests.len(), 1);

        let expected = serde_json::json!({
            "event": "subscribe",
            "pair": ["XBT/USD"],
            "subscription": {
                "name": "book",
                "depth": 100
            }
        });
        
        if let WsMessage::Text(text) = &requests[0] {
            let actual: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(actual, expected);
        } else {
            panic!("Expected WsMessage::Text");
        }
    }
}
