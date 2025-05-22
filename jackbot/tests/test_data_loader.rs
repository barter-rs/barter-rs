use Jackbot::backtest::{data_loader::JsonLinesLoader, market_data::MarketDataInMemory};
use Jackbot::error::JackbotError;
use chrono::Datelike;

#[tokio::test]
async fn test_json_lines_loader() -> Result<(), JackbotError> {
    let loader = JsonLinesLoader::<serde_json::Value>::new("jackbot/examples/data/binance_spot_trades_l1_btcusdt_ethusdt_solusdt.json");
    let data = MarketDataInMemory::from_loader(loader).await?;
    assert!(data.time_first_event().await?.year() >= 2025);
    Ok(())
}
