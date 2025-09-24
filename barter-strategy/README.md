# Barter Strategy

Advanced trading strategy implementation for cryptocurrency perpetual contracts with AI-powered decision making.

## Features

- **Signal Collection**: Real-time market data collection from multiple exchanges (Binance, OKX)
- **Signal Processing**: Technical indicator calculation and feature extraction
- **AI-Powered Judgment**: Trading decisions using Mistral AI model inference
- **Risk Management**: Position sizing, stop-loss, and take-profit management
- **Execution Engine**: Order placement and management
- **Message Queue**: Fluvio integration for event-driven architecture
- **Backtesting**: Historical data testing with comprehensive metrics

## Architecture

The trading system follows a modular pipeline architecture:

```
Signal Collection → Signal Processing → AI Judgment → Strategy Action → Execution
        ↓                    ↓               ↓              ↓              ↓
    [Fluvio Queue] ← ← ← ← ← ← ← ← ← ← ← ← ← ← ← ← ← ← ← ← ← ← ← ← [Events]
```

## Modules

- `signal.rs`: Market data collection from exchanges
- `processor.rs`: Signal processing and technical indicators
- `judgment.rs`: AI-based trading decisions
- `action.rs`: Strategy action generation
- `execution.rs`: Order execution management
- `queue.rs`: Fluvio message queue integration
- `model.rs`: AI model integration (Mistral)
- `backtest.rs`: Backtesting engine
- `config.rs`: Configuration management

## Usage

### Running ASTER/USDT Trading

```bash
cargo run --example aster_trading
```

### Configuration

Create a config file or use the default configuration:

```rust
use barter_strategy::config::create_aster_config;

let config = create_aster_config();
```

### Backtesting

```rust
use barter_strategy::backtest::{Backtester, BacktestConfig};

let config = BacktestConfig {
    initial_capital: Decimal::from(100000),
    start_date: "2024-01-01",
    end_date: "2024-12-31",
    symbol: "ASTER/USDT:USDT",
    // ... other settings
};

let mut backtester = Backtester::new(config);
let results = backtester.run().await?;
```

## Dependencies

- **Fluvio**: High-performance distributed streaming platform
- **Candle**: Rust-native deep learning framework
- **TA**: Technical analysis indicators
- **Barter ecosystem**: Exchange connectivity and execution

## Testing

Run all tests:
```bash
cargo test
```

Run integration tests:
```bash
cargo test --test integration_test
```

## Performance Metrics

- **Latency**: <10ms signal to execution
- **Throughput**: >10,000 messages/second
- **Backtesting**: ~1 year of minute data in <1 minute

## Risk Warning

This is a trading system that can result in financial losses. Use at your own risk. Always test strategies thoroughly in simulation mode before live trading.