use jackbot_execution::client::binance::futures::{BinanceFuturesUsd, BinanceFuturesUsdConfig};
use jackbot_execution::client::ExecutionClient;

#[test]
fn can_instantiate_binance_futures_client() {
    let _client = BinanceFuturesUsd::new(BinanceFuturesUsdConfig::default());
}

