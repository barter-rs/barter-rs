use crate::data::handler::Continuer;
use crate::data::market::MarketEvent;
use barter_data::client::ClientConfig;
use barter_data::client::binance::Binance;
use barter_data::ExchangeClient;
use barter_data::model::Candle;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;
use uuid::Uuid;

// Todo:
//  - Impl shutdown data feed - could use a terminus_rx field and let user work out the rest (eg/ REST API, etc)
//  - Can DateType be inferred by compiler when I create object, since i'll return
//  - Strings -> &str in consume_candles etc?
//  - Add builder method for LiveDataHandler
//  - Impl MarketGenerator / change the trait?
//  - Remove returning of Result<> for MarketGenerator, SignalGenerator
//  - Cannot return error from generate market because infinite loop would be faster
//    than candle interval, unless there is a relevant DataError variant. Use Option<MarketEvent>?
//  - Impl Display for ExchangeName to remove hack in generate_market() that uses Debug

#[derive(Debug, Deserialize)]
pub struct Config {
    pub client: ClientConfig,
    pub exchange: ExchangeName,
    pub symbol: String,
    pub interval: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ExchangeName { Binance, }

// enum DataType { Trade, Candle, Kline, }

pub struct LiveCandleHandler {
    pub exchange: ExchangeName,
    pub symbol: String,
    pub interval: String,
    pub data_stream: UnboundedReceiverStream<Candle>,
    pub can_continue: bool,
}

impl Continuer for LiveCandleHandler {
    fn should_continue(&self) -> bool {
        self.can_continue
    }
}

impl LiveCandleHandler {
    pub async fn generate_market(&mut self) -> Option<MarketEvent> {
        // Consume next candle if it's available
        let candle = match self.data_stream.next().await {
            Some(candle) => candle,
            _ => return None,
        };

        Some(
            MarketEvent {
                event_type: MarketEvent::EVENT_TYPE,
                trace_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                exchange: format!("{:?}", self.exchange.clone()),
                symbol: self.symbol.clone(),
                candle,
            }
        )
    }

    pub async fn new<Exchange>(cfg: &Config) -> Self
    where
        Exchange: ExchangeClient,
    {
        // Determine ExchangeClient instance & construct
        let mut exchange = match cfg.exchange {
            ExchangeName::Binance => Binance::new(ClientConfig {
                rate_limit_per_second: cfg.client.rate_limit_per_second,
            })
        }.await.unwrap();

        let data_stream = exchange
            .consume_candles(cfg.symbol.clone(), &*cfg.interval.clone())
            .await.unwrap();

        Self {
            exchange: cfg.exchange.clone(),
            symbol: cfg.symbol.clone(),
            interval: cfg.interval.clone(),
            data_stream,
            can_continue: true
        }
    }
}
