use crate::exchange::{
    BTC_USDT_PERP, BTC_USDT_SPOT, ETH_USDT_PERP, ETH_USDT_SPOT, MarketStreamTest,
};
use barter_instrument::exchange::ExchangeId;

#[tokio::test]
async fn binance_spot() {
    MarketStreamTest::builder(ExchangeId::BinanceSpot)
        .instruments([BTC_USDT_SPOT, ETH_USDT_SPOT])
        .build()
        .run()
        .await
        .unwrap()
}

#[tokio::test]
async fn binance_perpetual_usd() {
    MarketStreamTest::builder(ExchangeId::BinanceFuturesUsd)
        .instruments([BTC_USDT_PERP, ETH_USDT_PERP])
        .build()
        .run()
        .await
        .unwrap()
}
