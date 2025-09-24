use barter_strategy::{
    action::{ActionGenerator, RiskParameters},
    backtest::{Backtester, BacktestConfig},
    config::{create_aster_config, StrategyConfig},
    execution::StrategyExecution,
    judgment::SignalJudgment,
    model::MistralModel,
    processor::SignalProcessor,
    queue::FluvioQueue,
    signal::SignalCollector,
};
use barter_instrument::exchange::ExchangeId;
use chrono::Utc;
use rust_decimal::Decimal;
use tokio::time::{sleep, Duration};
use tracing::{error, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting ASTER/USDT:USDT Perpetual Trading System");

    // Load configuration
    let config = create_aster_config();
    info!("Configuration loaded");

    // Initialize Fluvio queue (will run in test mode if Fluvio is not available)
    let queue = FluvioQueue::new(config.queue.enabled).await?;
    queue.create_topics().await?;
    info!("Message queue initialized");

    // Initialize AI model
    let mut model = MistralModel::new(config.model.model_name.clone(), true)?; // Test mode
    model.load_model().await?;
    info!("AI model loaded");

    // Run backtest if enabled
    if config.backtest.enabled {
        run_backtest(&config).await?;
    } else {
        // Run live trading
        run_live_trading(config, queue, model).await?;
    }

    Ok(())
}

async fn run_live_trading(
    config: StrategyConfig,
    queue: FluvioQueue,
    model: MistralModel,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting live trading for ASTER/USDT:USDT");

    // Initialize components
    let symbol = config.trading.symbols[0].clone();
    let exchanges = vec![ExchangeId::BinanceFuturesUsd, ExchangeId::Okx];

    // Create signal collector
    let (signal_collector, mut signal_receiver) = SignalCollector::new(symbol.clone(), exchanges);

    // Start signal collection
    tokio::spawn(async move {
        if let Err(e) = signal_collector.start_collection().await {
            error!("Signal collection error: {}", e);
        }
    });

    // Create signal processor
    let mut signal_processor = SignalProcessor::new(symbol.clone(), 100);

    // Create signal judgment
    let signal_judgment = SignalJudgment::new(
        config.risk.risk_threshold,
        config.risk.confidence_threshold,
    );

    // Create action generator
    let mut action_generator = ActionGenerator::new(
        config.backtest.initial_capital,
        config.to_risk_parameters(),
    );

    // Create execution engine
    let mut execution = StrategyExecution::new(ExchangeId::BinanceFuturesUsd, true); // Test mode

    info!("All components initialized, starting main trading loop");

    // Get queue producers
    let signal_producer = queue.get_producer("aster-signals").await?;
    let decision_producer = queue.get_producer("aster-decisions").await?;
    let execution_producer = queue.get_producer("aster-executions").await?;

    // Main trading loop
    let mut trade_count = 0;
    let max_trades = 10; // Limit for demo

    while trade_count < max_trades {
        // Check for new signals
        if let Ok(signal) = signal_receiver.try_recv() {
            info!("Received signal: {:?}", signal.signal_type);

            // Send raw signal to queue
            if let Ok(bytes) = serde_json::to_vec(&signal) {
                signal_producer.send(&bytes).await?;
            }

            // Process signal
            match signal_processor.process(signal).await {
                Ok(processed) => {
                    info!("Signal processed: RSI={:?}, MACD={:?}",
                        processed.indicators.rsi,
                        processed.indicators.macd
                    );

                    // Make trading decision
                    match signal_judgment.judge(processed).await {
                        Ok(decision) => {
                            info!("Trading decision: {:?} (confidence: {:.2}%)",
                                decision.action,
                                decision.confidence * 100.0
                            );

                            // Send decision to queue
                            if let Ok(bytes) = serde_json::to_vec(&decision) {
                                decision_producer.send(&bytes).await?;
                            }

                            // Generate action
                            match action_generator.generate_action(decision).await {
                                Ok(Some(action)) => {
                                    info!("Executing action: {:?} {} @ {:?}",
                                        action.side,
                                        action.quantity,
                                        action.price
                                    );

                                    // Execute action
                                    match execution.execute(action).await {
                                        Ok(result) => {
                                            info!("Execution result: {:?}", result.status);

                                            // Send result to queue
                                            if let Ok(bytes) = serde_json::to_vec(&result) {
                                                execution_producer.send(&bytes).await?;
                                            }

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
        }

        // Sleep briefly to avoid busy waiting
        sleep(Duration::from_millis(100)).await;
    }

    info!("Trading session completed. Total trades: {}", trade_count);
    Ok(())
}

async fn run_backtest(config: &StrategyConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("Running backtest from {} to {}", config.backtest.start_date, config.backtest.end_date);

    let backtest_config = BacktestConfig {
        initial_capital: config.backtest.initial_capital,
        start_date: chrono::DateTime::parse_from_rfc3339(&config.backtest.start_date)?
            .with_timezone(&Utc),
        end_date: chrono::DateTime::parse_from_rfc3339(&config.backtest.end_date)?
            .with_timezone(&Utc),
        symbol: config.trading.symbols[0].clone(),
        risk_params: config.to_risk_parameters(),
        commission_rate: Decimal::from_str_exact("0.0004")?, // 0.04%
        slippage_rate: Decimal::from_str_exact("0.0001")?,   // 0.01%
        data_path: config.backtest.data_source.clone(),
    };

    let mut backtester = Backtester::new(backtest_config);
    let result = backtester.run().await?;

    // Print results
    println!("\n========== Backtest Results ==========");
    println!("Total Trades: {}", result.total_trades);
    println!("Winning Trades: {}", result.winning_trades);
    println!("Losing Trades: {}", result.losing_trades);
    println!("Win Rate: {:.2}%", result.win_rate * 100.0);
    println!("Total Return: {:.2}%", result.total_return * Decimal::from(100));
    println!("Max Drawdown: {:.2}%", result.max_drawdown * Decimal::from(100));
    println!("Sharpe Ratio: {:.2}", result.sharpe_ratio);
    println!("Profit Factor: {:.2}", result.profit_factor);

    println!("\n========== Trade Statistics ==========");
    println!("Average Win: {:.2}", result.statistics.avg_win);
    println!("Average Loss: {:.2}", result.statistics.avg_loss);
    println!("Best Trade: {:.2}", result.statistics.best_trade);
    println!("Worst Trade: {:.2}", result.statistics.worst_trade);
    println!("Max Consecutive Wins: {}", result.statistics.max_consecutive_wins);
    println!("Max Consecutive Losses: {}", result.statistics.max_consecutive_losses);

    // Export results if configured
    if config.backtest.export_results {
        backtester.export_results(&result, &config.backtest.results_path)?;
        info!("Results exported to {}", config.backtest.results_path);
    }

    Ok(())
}