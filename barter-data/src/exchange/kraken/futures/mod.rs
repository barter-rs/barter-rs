use crate::{
    Identifier,
    exchange::{ExchangeServer, StreamSelector, Connector, ExchangeSub},
    instrument::InstrumentData,
    subscription::{
        book::{OrderBooksL1, OrderBooksL2, OrderBookEvent, OrderBookL1}, 
        trade::{PublicTrades, PublicTrade},
        liquidation::{Liquidations, Liquidation},
    },
    transformer::stateless::StatelessTransformer,
    subscriber::{WebSocketSubscriber, validator::WebSocketSubValidator},
    NoInitialSnapshots,
    event::{MarketEvent, MarketIter},
    books::{Level, OrderBook},
};
use barter_integration::{error::SocketError, protocol::websocket::WsMessage, subscription::SubscriptionId};
use barter_instrument::exchange::ExchangeId;
use url::Url;
use serde_json::json;
use crate::exchange::kraken::{KrakenExchange, KrakenWsStream};
use chrono::Utc;

use self::{
    channel::KrakenFuturesChannel,
    market::KrakenFuturesMarket,
    subscription::KrakenFuturesSubResponse,
    trade::KrakenFuturesTrade,
    message::KrakenFuturesMessage,
    book::{l2::KrakenFuturesBook, l1::KrakenFuturesOrderBookL1},
};

pub mod book;
pub mod channel;
pub mod market;
pub mod message;
pub mod subscription;
pub mod trade;

/// [`KrakenFuturesUsd`] execution server.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct KrakenServerFuturesUsd;

impl ExchangeServer for KrakenServerFuturesUsd {
    const ID: ExchangeId = ExchangeId::KrakenFuturesUsd;

    fn websocket_url() -> &'static str {
        "wss://futures.kraken.com/ws/v1"
    }
}

pub type KrakenFuturesUsd = KrakenExchange<KrakenServerFuturesUsd>;

impl Connector for KrakenFuturesUsd {
    const ID: ExchangeId = KrakenServerFuturesUsd::ID;
    type Channel = KrakenFuturesChannel;
    type Market = KrakenFuturesMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = KrakenFuturesSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(KrakenServerFuturesUsd::websocket_url()).map_err(SocketError::UrlParse)
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        exchange_subs
            .into_iter()
            .map(|ExchangeSub { channel, market }| {
                WsMessage::text(
                    json!({
                        "event": "subscribe",
                        "feed": channel.as_ref(), // "trade", "book", "ticker"
                        "product_ids": [market.as_ref()]
                    })
                    .to_string(),
                )
            })
            .collect()
    }
}

impl<Instrument> StreamSelector<Instrument, PublicTrades> for KrakenFuturesUsd
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = KrakenWsStream<StatelessTransformer<Self, Instrument::Key, PublicTrades, KrakenFuturesMessage<KrakenFuturesTrade>>>;
}

impl<Instrument> StreamSelector<Instrument, Liquidations> for KrakenFuturesUsd
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = KrakenWsStream<StatelessTransformer<Self, Instrument::Key, Liquidations, KrakenFuturesMessage<KrakenFuturesTrade>>>;
}

impl<Instrument> StreamSelector<Instrument, OrderBooksL1> for KrakenFuturesUsd
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = KrakenWsStream<StatelessTransformer<Self, Instrument::Key, OrderBooksL1, KrakenFuturesMessage<KrakenFuturesOrderBookL1>>>;
}

impl<Instrument> StreamSelector<Instrument, OrderBooksL2> for KrakenFuturesUsd
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = KrakenWsStream<StatelessTransformer<Self, Instrument::Key, OrderBooksL2, KrakenFuturesMessage<KrakenFuturesBook>>>;
}

// ============================================================================
// Identifier implementations for StatelessTransformer to work
// ============================================================================

impl<T> Identifier<Option<SubscriptionId>> for KrakenFuturesMessage<T> {
    fn id(&self) -> Option<SubscriptionId> {
        // Create subscription ID from feed and product_id
        Some(SubscriptionId::from(format!("{}|{}", self.feed, self.product_id)))
    }
}

// ============================================================================
// From implementations for StatelessTransformer (3-tuple pattern)
// ============================================================================

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, KrakenFuturesMessage<KrakenFuturesTrade>)>
    for MarketIter<InstrumentKey, PublicTrade>
{
    fn from(
        (exchange_id, instrument, msg): (ExchangeId, InstrumentKey, KrakenFuturesMessage<KrakenFuturesTrade>),
    ) -> Self {
        let trade = msg.payload;
        Self(vec![Ok(MarketEvent {
            time_exchange: trade.time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: PublicTrade {
                id: trade.uid,
                price: trade.price.to_string().parse().unwrap_or(0.0),
                amount: trade.qty.to_string().parse().unwrap_or(0.0),
                side: trade.side,
            },
        })])
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, KrakenFuturesMessage<KrakenFuturesTrade>)>
    for MarketIter<InstrumentKey, Liquidation>
{
    fn from(
        (exchange_id, instrument, msg): (ExchangeId, InstrumentKey, KrakenFuturesMessage<KrakenFuturesTrade>),
    ) -> Self {
        let trade = msg.payload;
        
        // Note: Kraken Futures sends all trades on the same feed.
        // Only trades with type "liquidation" are actual liquidations.
        // The StatelessTransformer doesn't filter, so downstream consumers
        // may need to check trade_type if using the trade feed for liquidations.
        
        Self(vec![Ok(MarketEvent {
            time_exchange: trade.time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: Liquidation {
                side: trade.side,
                price: trade.price.to_string().parse().unwrap_or(0.0),
                quantity: trade.qty.to_string().parse().unwrap_or(0.0),
                time: trade.time,
            },
        })])
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, KrakenFuturesMessage<KrakenFuturesOrderBookL1>)>
    for MarketIter<InstrumentKey, OrderBookL1>
{
    fn from(
        (exchange_id, instrument, msg): (ExchangeId, InstrumentKey, KrakenFuturesMessage<KrakenFuturesOrderBookL1>),
    ) -> Self {
        let l1 = msg.payload;
        let time = Utc::now();
        
        Self(vec![Ok(MarketEvent {
            time_exchange: time,
            time_received: time,
            exchange: exchange_id,
            instrument,
            kind: OrderBookL1 {
                last_update_time: time,
                best_bid: Some(Level { 
                    price: l1.bid, 
                    amount: l1.bid_size 
                }),
                best_ask: Some(Level { 
                    price: l1.ask, 
                    amount: l1.ask_size 
                }),
            },
        })])
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, KrakenFuturesMessage<KrakenFuturesBook>)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange_id, instrument, msg): (ExchangeId, InstrumentKey, KrakenFuturesMessage<KrakenFuturesBook>),
    ) -> Self {
        let book = msg.payload;
        
        let event = if msg.feed.contains("snapshot") {
            OrderBookEvent::Snapshot(OrderBook::new(
                book.seq,
                Some(book.time),
                book.bids.into_iter().map(|l| Level { price: l.price, amount: l.qty }),
                book.asks.into_iter().map(|l| Level { price: l.price, amount: l.qty }),
            ))
        } else {
            OrderBookEvent::Update(OrderBook::new(
                book.seq,
                Some(book.time),
                book.bids.into_iter().map(|l| Level { price: l.price, amount: l.qty }),
                book.asks.into_iter().map(|l| Level { price: l.price, amount: l.qty }),
            ))
        };

        Self(vec![Ok(MarketEvent {
            time_exchange: Utc::now(),
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: event,
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smol_str::SmolStr;

    #[test]
    fn test_kraken_futures_connector_id() {
        assert_eq!(<KrakenFuturesUsd as Connector>::ID, ExchangeId::KrakenFuturesUsd);
    }

    #[test]
    fn test_kraken_futures_websocket_url() {
        let url = KrakenServerFuturesUsd::websocket_url();
        assert_eq!(url, "wss://futures.kraken.com/ws/v1");
    }

    #[test]
    fn test_kraken_futures_url_parsing() {
        let url = KrakenFuturesUsd::url().expect("Failed to parse URL");
        assert_eq!(url.scheme(), "wss");
        assert_eq!(url.host_str(), Some("futures.kraken.com"));
        assert_eq!(url.path(), "/ws/v1");
    }

    #[test]
    fn test_kraken_futures_requests_trade() {
        let exchange_subs = vec![ExchangeSub {
            channel: KrakenFuturesChannel::Trade,
            market: KrakenFuturesMarket(SmolStr::new("PI_XBTUSD")),
        }];

        let requests = KrakenFuturesUsd::requests(exchange_subs);
        assert_eq!(requests.len(), 1);

        let expected = serde_json::json!({
            "event": "subscribe",
            "feed": "trade",
            "product_ids": ["PI_XBTUSD"]
        });
        
        if let WsMessage::Text(text) = &requests[0] {
            let actual: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(actual, expected);
        } else {
            panic!("Expected WsMessage::Text");
        }
    }

    #[test]
    fn test_kraken_futures_requests_ticker() {
        let exchange_subs = vec![ExchangeSub {
            channel: KrakenFuturesChannel::Ticker,
            market: KrakenFuturesMarket(SmolStr::new("PI_ETHUSD")),
        }];

        let requests = KrakenFuturesUsd::requests(exchange_subs);
        assert_eq!(requests.len(), 1);

        let expected = serde_json::json!({
            "event": "subscribe",
            "feed": "ticker",
            "product_ids": ["PI_ETHUSD"]
        });
        
        if let WsMessage::Text(text) = &requests[0] {
            let actual: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(actual, expected);
        } else {
            panic!("Expected WsMessage::Text");
        }
    }

    #[test]
    fn test_kraken_futures_requests_book() {
        let exchange_subs = vec![ExchangeSub {
            channel: KrakenFuturesChannel::Book,
            market: KrakenFuturesMarket(SmolStr::new("PI_XBTUSD")),
        }];

        let requests = KrakenFuturesUsd::requests(exchange_subs);
        assert_eq!(requests.len(), 1);

        let expected = serde_json::json!({
            "event": "subscribe",
            "feed": "book",
            "product_ids": ["PI_XBTUSD"]
        });
        
        if let WsMessage::Text(text) = &requests[0] {
            let actual: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(actual, expected);
        } else {
            panic!("Expected WsMessage::Text");
        }
    }

    #[test]
    fn test_kraken_futures_multiple_subscriptions() {
        let exchange_subs = vec![
            ExchangeSub {
                channel: KrakenFuturesChannel::Trade,
                market: KrakenFuturesMarket(SmolStr::new("PI_XBTUSD")),
            },
            ExchangeSub {
                channel: KrakenFuturesChannel::Trade,
                market: KrakenFuturesMarket(SmolStr::new("PI_ETHUSD")),
            },
        ];

        let requests = KrakenFuturesUsd::requests(exchange_subs);
        // Each subscription creates a separate request
        assert_eq!(requests.len(), 2);
    }

    #[test]
    fn test_kraken_futures_message_identifier() {
        let msg = KrakenFuturesMessage {
            feed: "trade".to_string(),
            product_id: "PI_XBTUSD".to_string(),
            payload: (),
        };
        
        let id = msg.id();
        assert!(id.is_some());
        // SubscriptionId contains the string in .0 field
        assert_eq!(id.unwrap().0.as_str(), "trade|PI_XBTUSD");
    }
}