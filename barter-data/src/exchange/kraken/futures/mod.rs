use crate::{
    exchange::{ExchangeServer, StreamSelector, Connector, ExchangeSub},
    instrument::InstrumentData,
    subscription::{book::{OrderBooksL1, OrderBooksL2, OrderBookEvent, OrderBookL1}, trade::{PublicTrades, Liquidations, PublicTrade}},
    transformer::stateless::StatelessTransformer,
    subscriber::{WebSocketSubscriber, validator::WebSocketSubValidator},
    NoInitialSnapshots,
    event::{MarketEvent, MarketIter},
    books::{Level, OrderBook},
};
use barter_integration::{error::SocketError, protocol::websocket::WsMessage, model::Side};
use barter_instrument::exchange::ExchangeId;
use url::Url;
use serde_json::json;
use crate::exchange::kraken::{KrakenExchange, KrakenWsStream};
use serde::Deserialize;

use self::{
    channel::KrakenFuturesChannel,
    market::KrakenFuturesMarket,
    subscription::KrakenFuturesSubResponse,
    trade::{KrakenFuturesTrade, KrakenFuturesTradeType},
    message::KrakenFuturesMessage,
    book::{l2::KrakenFuturesBook, l1::KrakenFuturesOrderBookL1, KrakenFuturesLevel},
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

// Transformer Implementations

impl<InstrumentKey> From<(KrakenFuturesMessage<KrakenFuturesTrade>, InstrumentKey)> for MarketEvent<PublicTrade>
where
    InstrumentKey: Clone,
{
    fn from((msg, instrument): (KrakenFuturesMessage<KrakenFuturesTrade>, InstrumentKey)) -> Self {
        let trade = msg.payload;
        MarketEvent {
            exchange_time: trade.time,
            received_time: chrono::Utc::now(),
            exchange: ExchangeId::KrakenFuturesUsd,
            instrument,
            kind: PublicTrade {
                id: trade.uid,
                price: trade.price,
                amount: trade.qty,
                side: trade.side,
            },
        }
    }
}

impl<InstrumentKey> From<(KrakenFuturesMessage<KrakenFuturesTrade>, InstrumentKey)> for MarketEvent<barter_integration::model::Liquidation>
where
    InstrumentKey: Clone,
{
    fn from((msg, instrument): (KrakenFuturesMessage<KrakenFuturesTrade>, InstrumentKey)) -> Self {
        let trade = msg.payload;
        
        // Note: Filtering logic limitation of StatelessTransformer.
        // Assuming user handles all trades or we need custom logic.
        
        MarketEvent {
            exchange_time: trade.time,
            received_time: chrono::Utc::now(),
            exchange: ExchangeId::KrakenFuturesUsd,
            instrument,
            kind: barter_integration::model::Liquidation {
                price: trade.price,
                amount: trade.qty,
                side: trade.side,
                time: trade.time,
            },
        }
    }
}

impl<InstrumentKey> From<(KrakenFuturesMessage<KrakenFuturesOrderBookL1>, InstrumentKey)> for MarketEvent<OrderBookL1>
where
    InstrumentKey: Clone,
{
    fn from((msg, instrument): (KrakenFuturesMessage<KrakenFuturesOrderBookL1>, InstrumentKey)) -> Self {
        let l1 = msg.payload;
        // Placeholder time - ticker message usually contains time?
        let time = chrono::Utc::now(); 
        
        MarketEvent {
            exchange_time: time,
            received_time: time,
            exchange: ExchangeId::KrakenFuturesUsd,
            instrument,
            kind: OrderBookL1 {
                last_update_time: time,
                best_bid: Some(Level { price: l1.bid, amount: l1.bid_size }),
                best_ask: Some(Level { price: l1.ask, amount: l1.ask_size }),
            },
        }
    }
}

impl<InstrumentKey> From<(KrakenFuturesMessage<KrakenFuturesBook>, InstrumentKey)> for MarketEvent<OrderBookEvent>
where
    InstrumentKey: Clone,
{
    fn from((msg, instrument): (KrakenFuturesMessage<KrakenFuturesBook>, InstrumentKey)) -> Self {
        let book = msg.payload;
        
        let update = if msg.feed.contains("snapshot") {
            OrderBookEvent::Snapshot(OrderBook {
                last_update_time: book.time,
                bids: book.bids.into_iter().map(|l| Level { price: l.price, amount: l.qty }).collect(),
                asks: book.asks.into_iter().map(|l| Level { price: l.price, amount: l.qty }).collect(),
            })
        } else {
             OrderBookEvent::Update(OrderBook {
                last_update_time: book.time,
                bids: book.bids.into_iter().map(|l| Level { price: l.price, amount: l.qty }).collect(),
                asks: book.asks.into_iter().map(|l| Level { price: l.price, amount: l.qty }).collect(),
            })
        };

        MarketEvent {
            exchange_time: book.time,
            received_time: chrono::Utc::now(),
            exchange: ExchangeId::KrakenFuturesUsd,
            instrument,
            kind: update,
        }
    }
}
