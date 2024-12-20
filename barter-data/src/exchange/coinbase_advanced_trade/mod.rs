use crate::{
    exchange::{Connector, ExchangeSub, StreamSelector},
    instrument::InstrumentData,
    subscriber::{validator::WebSocketSubValidator, WebSocketSubscriber},
    subscription::trade::PublicTrades,
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream, NoInitialSnapshots,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{error::SocketError, protocol::websocket::WsMessage};
use barter_macro::{DeExchange, SerExchange};
use serde_json::json;
use url::Url;
use crate::exchange::coinbase_advanced_trade::candles::CandleEvent;
use crate::exchange::coinbase_advanced_trade::channel::CoinbaseInternationalChannel;
use crate::exchange::coinbase_advanced_trade::level2::Level2Event;
use crate::exchange::coinbase_advanced_trade::market::CoinbaseInternationalMarket;
use crate::exchange::coinbase_advanced_trade::market_trades::{MarketTradeEvent};
use crate::exchange::coinbase_advanced_trade::message::CoinbaseInternationalMessage;
use crate::exchange::coinbase_advanced_trade::subscription::CoinbaseInternationalSubResponse;
use crate::exchange::coinbase_advanced_trade::ticker::TickerEvent;
use crate::subscription::book::{OrderBooksL1, OrderBooksL2};
use crate::subscription::candle::Candles;

pub mod channel;
pub mod market;
pub mod subscription;
pub mod message;
mod market_trades;
mod ticker;
mod level2;
mod candles;
/// \[`CoinbaseInternational`\] server base url.
///
/// See docs: <https://docs.cdp.coinbase.com/advanced-trade/docs/ws-overview>
pub const BASE_URL_COINBASE_INTERNATIONAL: &str = "wss://advanced-trade-ws.coinbase.com";

/// \[`CoinbaseInternational`\] exchange.
///
/// See docs: <https://docs.cdp.coinbase.com/advanced-trade/docs/ws-overview>
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, DeExchange, SerExchange,
)]
pub struct CoinbaseInternational;

impl Connector for CoinbaseInternational {
    const ID: ExchangeId = ExchangeId::CoinbaseInternational;
    type Channel = CoinbaseInternationalChannel;
    type Market = CoinbaseInternationalMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = CoinbaseInternationalSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(BASE_URL_COINBASE_INTERNATIONAL).map_err(SocketError::UrlParse)
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        exchange_subs
            .into_iter()
            .map(|ExchangeSub { channel, market }| {
                WsMessage::Text(
                    json!({
                        "type": "subscribe",
                        "product_ids": [market.as_ref()],
                        "channel": channel.as_ref(),
                    })
                        .to_string(),
                )
            })
            .collect()
    }
}

impl<Instrument> StreamSelector<Instrument, PublicTrades> for CoinbaseInternational
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream =
    ExchangeWsStream<StatelessTransformer<Self, Instrument::Key, PublicTrades, CoinbaseInternationalMessage<MarketTradeEvent>>>;
}


impl<Instrument> StreamSelector<Instrument, OrderBooksL1> for CoinbaseInternational
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream =
    ExchangeWsStream<StatelessTransformer<Self, Instrument::Key, OrderBooksL1, CoinbaseInternationalMessage<TickerEvent>>>;
}


impl<Instrument> StreamSelector<Instrument, OrderBooksL2> for CoinbaseInternational
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream =
    ExchangeWsStream<StatelessTransformer<Self, Instrument::Key, OrderBooksL2, CoinbaseInternationalMessage<Level2Event>>>;
}

impl<Instrument> StreamSelector<Instrument, Candles> for CoinbaseInternational
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream =
    ExchangeWsStream<StatelessTransformer<Self, Instrument::Key, Candles, CoinbaseInternationalMessage<CandleEvent>>>;
}