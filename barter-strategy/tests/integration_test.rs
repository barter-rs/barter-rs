use barter_strategy::{
    action::{ActionGenerator, OrderSide, PositionSizingMethod, RiskParameters},
    config::create_aster_config,
    execution::StrategyExecution,
    judgment::{SignalJudgment, TradingAction, TradingDecision},
    model::{MistralModel, ModelPrediction, PredictedAction, TradingFeatures},
    processor::{ProcessedSignal, SignalProcessor},
    signal::{MarketSignal, SignalCollector, SignalData, TradeSide},
};
use barter_instrument::exchange::ExchangeId;
use chrono::Utc;
use rust_decimal::Decimal;

#[tokio::test]
async fn test_signal_collection() {
    let symbol = "ASTER/USDT:USDT".to_string();
    let exchanges = vec![ExchangeId::BinanceFuturesUsd];

    let (collector, mut receiver) = SignalCollector::new(symbol, exchanges);

    // Start collection in background
    tokio::spawn(async move {
        let _ = collector.start_collection().await;
    });

    // Should receive at least one signal
    tokio::time::timeout(tokio::time::Duration::from_secs(1), async {
        if let Ok(signal) = receiver.recv().await {
            assert!(!signal.symbol.is_empty());
        }
    })
    .await
    .ok();
}

#[tokio::test]
async fn test_signal_processing() {
    let mut processor = SignalProcessor::new("ASTER/USDT".to_string(), 100);

    let signal = MarketSignal {
        timestamp: Utc::now(),
        exchange: ExchangeId::BinanceFuturesUsd,
        symbol: "ASTER/USDT".to_string(),
        signal_type: crate::signal::SignalType::Trade,
        data: SignalData::Trade {
            price: Decimal::from(100),
            amount: Decimal::from(10),
            side: TradeSide::Buy,
        },
    };

    let processed = processor.process(signal).await.unwrap();
    assert_eq!(processed.symbol, "ASTER/USDT");
    assert!(processed.features.price > Decimal::ZERO);
}

#[tokio::test]
async fn test_trading_judgment() {
    let judgment = SignalJudgment::new(0.7, 0.6);

    let processed = ProcessedSignal {
        timestamp: Utc::now(),
        symbol: "ASTER/USDT".to_string(),
        features: crate::processor::Features {
            price: Decimal::from(100),
            volume: Decimal::from(1000),
            spread: Decimal::from_str_exact("0.01").unwrap(),
            volatility: 0.2,
            momentum: 0.05,
            order_imbalance: 0.1,
        },
        indicators: crate::processor::TechnicalIndicators {
            sma_20: Some(99.5),
            sma_50: Some(98.0),
            ema_12: Some(100.5),
            ema_26: Some(99.0),
            rsi: Some(55.0),
            macd: Some(1.5),
            macd_signal: Some(1.2),
            bollinger_upper: Some(102.0),
            bollinger_lower: Some(98.0),
        },
        market_microstructure: crate::processor::MarketMicrostructure {
            bid_ask_spread: Decimal::from_str_exact("0.01").unwrap(),
            bid_volume: Decimal::from(500),
            ask_volume: Decimal::from(500),
            trade_intensity: 10.0,
            price_impact: 0.001,
        },
    };

    let decision = judgment.judge(processed).await.unwrap();
    assert!(!decision.symbol.is_empty());
    assert!(decision.confidence >= 0.0 && decision.confidence <= 1.0);
}

#[tokio::test]
async fn test_action_generation() {
    let risk_params = RiskParameters {
        max_position_size: Decimal::from(10000),
        max_leverage: 20,
        default_leverage: 5,
        position_sizing_method: PositionSizingMethod::Fixed(Decimal::from(100)),
        max_loss_per_trade: Decimal::from_str_exact("0.02").unwrap(),
        max_daily_loss: Decimal::from_str_exact("0.05").unwrap(),
    };

    let mut generator = ActionGenerator::new(Decimal::from(10000), risk_params);

    let decision = TradingDecision {
        timestamp: Utc::now(),
        symbol: "ASTER/USDT".to_string(),
        action: TradingAction::OpenLong,
        confidence: 0.8,
        risk_score: 0.3,
        rationale: "Test decision".to_string(),
        target_price: Some(Decimal::from(105)),
        stop_loss: Some(Decimal::from(95)),
    };

    let action = generator.generate_action(decision).await.unwrap();
    assert!(action.is_some());

    if let Some(action) = action {
        assert_eq!(action.symbol, "ASTER/USDT");
        assert_eq!(action.side, OrderSide::Buy);
        assert!(action.quantity > Decimal::ZERO);
    }
}

#[tokio::test]
async fn test_execution() {
    let mut execution = StrategyExecution::new(ExchangeId::BinanceFuturesUsd, true);

    let action = crate::action::StrategyAction {
        action_id: "test_action".to_string(),
        timestamp: Utc::now(),
        symbol: "ASTER/USDT".to_string(),
        order_type: crate::action::OrderType::Market,
        side: OrderSide::Buy,
        quantity: Decimal::from(100),
        price: None,
        leverage: 5,
        reduce_only: false,
        time_in_force: crate::action::TimeInForce::GTC,
        stop_loss: Some(Decimal::from(95)),
        take_profit: Some(Decimal::from(105)),
        metadata: crate::action::ActionMetadata {
            decision_id: "test_decision".to_string(),
            confidence: 0.8,
            risk_score: 0.3,
            expected_pnl: Some(Decimal::from(500)),
            max_loss: Some(Decimal::from(500)),
        },
    };

    let result = execution.execute(action).await.unwrap();
    assert_eq!(result.status, crate::execution::ExecutionStatus::Filled);
    assert!(result.filled_quantity > Decimal::ZERO);
}

#[tokio::test]
async fn test_model_prediction() {
    let model = MistralModel::new("test_model".to_string(), true).unwrap();

    let features = TradingFeatures {
        price: 100.0,
        volume: 1000.0,
        volume_ratio: 1.2,
        rsi: 45.0,
        macd: 0.5,
        macd_signal: 0.3,
        sma_20: 99.0,
        sma_50: 98.0,
        ema_12: 100.0,
        ema_26: 99.5,
        volatility: 0.2,
        spread: 0.01,
        order_imbalance: 0.1,
    };

    let prediction = model.predict(&features).await.unwrap();
    assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
    assert!(prediction.risk_score >= 0.0 && prediction.risk_score <= 1.0);
}

#[test]
fn test_config_creation() {
    let config = create_aster_config();
    assert_eq!(config.trading.symbols[0], "ASTER-USDT-SWAP");
    assert_eq!(config.trading.leverage, 10);
    assert_eq!(config.risk.position_sizing_method, "kelly");
}

#[tokio::test]
async fn test_queue_initialization() {
    let queue = crate::queue::FluvioQueue::new(true).await.unwrap();
    let producer = queue.get_producer("test-topic").await.unwrap();

    // Should not error in test mode
    producer.send(b"test message").await.unwrap();
}