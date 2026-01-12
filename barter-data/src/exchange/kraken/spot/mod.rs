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
