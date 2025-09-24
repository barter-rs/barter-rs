#!/bin/bash
set -e

# Function to wait for a service to be ready
wait_for_service() {
    local host=$1
    local port=$2
    local service_name=$3
    local max_attempts=30
    local attempt=1

    echo "Waiting for $service_name to be ready..."

    while [ $attempt -le $max_attempts ]; do
        if nc -z "$host" "$port" 2>/dev/null; then
            echo "$service_name is ready!"
            return 0
        fi
        echo "Attempt $attempt/$max_attempts: $service_name not ready yet..."
        sleep 2
        attempt=$((attempt + 1))
    done

    echo "ERROR: $service_name failed to start after $max_attempts attempts"
    return 1
}

# Wait for dependencies
if [ "$WAIT_FOR_FLUVIO" = "true" ]; then
    wait_for_service "${FLUVIO_HOST:-fluvio}" "${FLUVIO_PORT:-9003}" "Fluvio"
fi

if [ "$WAIT_FOR_REDIS" = "true" ]; then
    wait_for_service "${REDIS_HOST:-redis}" "${REDIS_PORT:-6379}" "Redis"
fi

if [ "$WAIT_FOR_POSTGRES" = "true" ]; then
    wait_for_service "${POSTGRES_HOST:-postgres}" "${POSTGRES_PORT:-5432}" "PostgreSQL"
fi

# Configure trading mode
case "$TRADING_MODE" in
    "live")
        echo "Starting in LIVE trading mode"
        if [ -z "$EXCHANGE_API_KEY" ] || [ -z "$EXCHANGE_API_SECRET" ]; then
            echo "ERROR: Exchange API credentials not set for live trading"
            exit 1
        fi
        ;;
    "paper")
        echo "Starting in PAPER trading mode"
        ;;
    "backtest")
        echo "Starting in BACKTEST mode"
        ;;
    *)
        echo "Starting in TEST mode (default)"
        export TRADING_MODE="test"
        ;;
esac

# Create necessary directories
mkdir -p /opt/barter/logs /opt/barter/data /opt/barter/config

# Generate config if not exists
if [ ! -f "/opt/barter/config/strategy.json" ]; then
    echo "Generating default configuration..."
    cat > /opt/barter/config/strategy.json <<EOF
{
  "exchanges": [
    {
      "exchange_id": "binance_futures",
      "api_key": "${EXCHANGE_API_KEY:-}",
      "api_secret": "${EXCHANGE_API_SECRET:-}",
      "test_mode": $([ "$TRADING_MODE" = "live" ] && echo "false" || echo "true")
    }
  ],
  "trading": {
    "symbols": ["${TRADING_SYMBOL:-ASTER-USDT-SWAP}"],
    "leverage": ${TRADING_LEVERAGE:-10},
    "order_type": "market",
    "time_in_force": "GTC",
    "enable_stop_loss": true,
    "enable_take_profit": true,
    "stop_loss_pct": ${STOP_LOSS_PCT:-0.03},
    "take_profit_pct": ${TAKE_PROFIT_PCT:-0.06}
  },
  "risk": {
    "max_position_size": ${MAX_POSITION_SIZE:-50000},
    "max_leverage": ${MAX_LEVERAGE:-20},
    "position_sizing_method": "${POSITION_SIZING:-kelly}",
    "sizing_parameter": ${SIZING_PARAMETER:-0.25},
    "max_loss_per_trade": ${MAX_LOSS_PER_TRADE:-0.03},
    "max_daily_loss": ${MAX_DAILY_LOSS:-0.10},
    "confidence_threshold": ${CONFIDENCE_THRESHOLD:-0.65},
    "risk_threshold": ${RISK_THRESHOLD:-0.75}
  },
  "model": {
    "model_name": "${MODEL_NAME:-mistral-7b-instruct}",
    "model_path": "/opt/barter/models",
    "use_gpu": ${USE_GPU:-false},
    "batch_size": ${MODEL_BATCH_SIZE:-4},
    "prediction_threshold": ${PREDICTION_THRESHOLD:-0.7}
  },
  "queue": {
    "enabled": ${QUEUE_ENABLED:-true},
    "fluvio_url": "${FLUVIO_URL:-localhost:9003}",
    "topics": [
      "aster-market-data",
      "aster-signals",
      "aster-decisions",
      "aster-executions"
    ],
    "consumer_group": "aster-strategy"
  },
  "backtest": {
    "enabled": $([ "$TRADING_MODE" = "backtest" ] && echo "true" || echo "false"),
    "start_date": "${BACKTEST_START:-2024-01-01T00:00:00Z}",
    "end_date": "${BACKTEST_END:-2024-12-31T23:59:59Z}",
    "initial_capital": ${INITIAL_CAPITAL:-100000},
    "data_source": "/opt/barter/data/historical.csv",
    "export_results": true,
    "results_path": "/opt/barter/data/backtest_results.csv"
  }
}
EOF
fi

# Log startup information
echo "=================================="
echo "Barter Trading System Starting"
echo "=================================="
echo "Trading Mode: $TRADING_MODE"
echo "Symbol: ${TRADING_SYMBOL:-ASTER-USDT-SWAP}"
echo "Leverage: ${TRADING_LEVERAGE:-10}x"
echo "Log Level: $RUST_LOG"
echo "Config: /opt/barter/config/strategy.json"
echo "=================================="

# Execute the main command
exec "$@"