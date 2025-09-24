use barter_data::{
    exchange::binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
    streams::Streams,
    subscription::{book::OrderBooksL1, trade::PublicTrades},
};
use barter_execution::{
    client::{ClientKind, ExecutionClient},
    order::{Order, OrderKind, RequestOpen},
};
use barter_instrument::{exchange::ExchangeId, instrument::Instrument};
use barter_integration::model::instrument::InstrumentData;
use barter_strategy::{
    action::{ActionGenerator, RiskParameters},
    execution::StrategyExecution,
    judgment::SignalJudgment,
    processor::SignalProcessor,
    signal::{MarketSignal, SignalCollector},
};
use chrono::Utc;
use rust_decimal::Decimal;
use std::env;
use tracing::{error, info, warn};
use tracing_subscriber::FmtSubscriber;

/// Binance Testnet Trading Example
/// Uses the provided testnet API keys for safe testing
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("\n========================================");
    println!("   Binance Testnet Trading System");
    println!("========================================\n");

    info!("Starting Binance Testnet Trading System");
    info!("This uses TESTNET - no real funds involved!");

    // Testnet credentials (hardcoded for this example)
    let api_key = "Wt104kkmijNETENuP4hpJfnGLZxjcjhpH7cYVckIvGAeeI6vxd24Vf8zGKs4lznM";
    let api_secret = "q7MCl5Fp3tILTDsoVA7rG6WzzV2lscHYWsYVp65RYZaXI5dnDGMqXMKDkaniP2Wx";

    // Test connection first
    if !test_connection(api_key, api_secret).await? {
        error!("Failed to connect to Binance testnet");
        return Ok(());
    }

    info!("✅ Connected to Binance Testnet successfully!");

    // Trading configuration
    let symbol = "BTCUSDT";  // Most liquid pair on testnet
    let initial_capital = Decimal::from(10000);  // Virtual capital

    // Initialize components
    let mut signal_processor = SignalProcessor::new(symbol.to_string(), 100);
    let signal_judgment = SignalJudgment::new(0.7, 0.6);

    let risk_params = RiskParameters {
        max_position_size: Decimal::from(1000),
        max_leverage: 10,
        default_leverage: 3,
        position_sizing_method: barter_strategy::action::PositionSizingMethod::PercentageOfCapital(
            Decimal::from_str_exact("0.1").unwrap()
        ),
        max_loss_per_trade: Decimal::from_str_exact("0.02").unwrap(),
        max_daily_loss: Decimal::from_str_exact("0.05").unwrap(),
    };

    let mut action_generator = ActionGenerator::new(initial_capital, risk_params);
    let mut execution = StrategyExecution::new(ExchangeId::BinanceSpot, true);

    info!("Starting market data collection for {}", symbol);

    // Create data stream for BTCUSDT
    let mut trade_count = 0;
    let max_trades = 5;  // Limit for demo

    // Simulate receiving market signals
    // In production, this would be real WebSocket streams
    while trade_count < max_trades {
        // Generate a test signal
        let signal = create_test_signal(symbol).await?;
        info!("Received signal: {:?}", signal.signal_type);

        // Process the signal
        match signal_processor.process(signal).await {
            Ok(processed) => {
                info!(
                    "Processed signal - Price: {}, Volume: {}, RSI: {:?}",
                    processed.features.price,
                    processed.features.volume,
                    processed.indicators.rsi
                );

                // Make trading decision
                match signal_judgment.judge(processed).await {
                    Ok(decision) => {
                        info!(
                            "Trading decision: {:?} (confidence: {:.2}%)",
                            decision.action,
                            decision.confidence * 100.0
                        );

                        // Generate action
                        match action_generator.generate_action(decision).await {
                            Ok(Some(action)) => {
                                info!(
                                    "Generated action: {:?} {} @ {:?}",
                                    action.side, action.quantity, action.price
                                );

                                // Execute on testnet
                                match execution.execute(action).await {
                                    Ok(result) => {
                                        info!("✅ Execution result: {:?}", result.status);
                                        info!("  Order ID: {}", result.order_id);
                                        info!("  Filled: {}", result.filled_quantity);
                                        trade_count += 1;
                                    }
                                    Err(e) => error!("Execution error: {}", e),
                                }
                            }
                            Ok(None) => info!("No action required"),
                            Err(e) => error!("Action generation error: {}", e),
                        }
                    }
                    Err(e) => error!("Judgment error: {}", e),
                }
            }
            Err(e) => error!("Processing error: {}", e),
        }

        // Wait before next iteration
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }

    // Summary
    println!("\n========================================");
    println!("         Trading Session Complete");
    println!("========================================");
    println!("Total trades executed: {}", trade_count);
    println!("Mode: Binance Testnet (Paper Trading)");
    println!("\nNext steps:");
    println!("1. Check Grafana dashboard: http://localhost:3001");
    println!("2. Review logs: docker-compose -f docker-compose.testnet.yml logs");
    println!("3. Query database for trades");

    Ok(())
}

async fn test_connection(api_key: &str, api_secret: &str) -> Result<bool, Box<dyn std::error::Error>> {
    info!("Testing Binance testnet connection...");

    let client = reqwest::Client::new();
    let base_url = "https://testnet.binance.vision";

    // Test public endpoint
    let response = client
        .get(format!("{}/api/v3/ping", base_url))
        .send()
        .await?;

    if !response.status().is_success() {
        error!("Failed to reach Binance testnet");
        return Ok(false);
    }

    // Test authenticated endpoint
    let timestamp = chrono::Utc::now().timestamp_millis();
    let query = format!("timestamp={}", timestamp);

    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(api_secret.as_bytes())?;
    mac.update(query.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    let response = client
        .get(format!("{}/api/v3/account?{}&signature={}", base_url, query, signature))
        .header("X-MBX-APIKEY", api_key)
        .send()
        .await?;

    if response.status().is_success() {
        let body = response.text().await?;
        if body.contains("balances") {
            info!("✅ Authentication successful");

            // Parse and show balances
            if let Ok(account) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(balances) = account["balances"].as_array() {
                    println!("\nTestnet Account Balances:");
                    for balance in balances {
                        let asset = balance["asset"].as_str().unwrap_or("");
                        let free = balance["free"].as_str().unwrap_or("0");
                        let free_val: f64 = free.parse().unwrap_or(0.0);
                        if free_val > 0.0 {
                            println!("  {}: {}", asset, free);
                        }
                    }
                }
            }
            return Ok(true);
        }
    }

    warn!("Authentication failed - check API keys");
    Ok(false)
}

async fn create_test_signal(symbol: &str) -> Result<MarketSignal, Box<dyn std::error::Error>> {
    use barter_strategy::signal::{SignalData, SignalType, TradeSide};

    // Get real price from testnet
    let client = reqwest::Client::new();
    let response = client
        .get(format!("https://testnet.binance.vision/api/v3/ticker/price?symbol={}", symbol))
        .send()
        .await?;

    let ticker: serde_json::Value = response.json().await?;
    let price = ticker["price"]
        .as_str()
        .unwrap_or("50000")
        .parse::<f64>()
        .unwrap_or(50000.0);

    // Create a market signal with real testnet data
    Ok(MarketSignal {
        timestamp: Utc::now(),
        exchange: ExchangeId::BinanceSpot,
        symbol: symbol.to_string(),
        signal_type: SignalType::Trade,
        data: SignalData::Trade {
            price: Decimal::from_f64_retain(price).unwrap_or(Decimal::from(50000)),
            amount: Decimal::from(1),
            side: if rand::random::<bool>() {
                TradeSide::Buy
            } else {
                TradeSide::Sell
            },
        },
    })
}

// Helper function to display trade results
fn display_trade_results(trades: Vec<serde_json::Value>) {
    println!("\n📊 Trade Results:");
    println!("================");
    for (i, trade) in trades.iter().enumerate() {
        println!("\nTrade #{}:", i + 1);
        println!("  Symbol: {}", trade["symbol"].as_str().unwrap_or(""));
        println!("  Side: {}", trade["side"].as_str().unwrap_or(""));
        println!("  Price: {}", trade["price"].as_str().unwrap_or(""));
        println!("  Quantity: {}", trade["quantity"].as_str().unwrap_or(""));
        println!("  Status: {}", trade["status"].as_str().unwrap_or(""));
    }
}