# API Rate Limiting

Jackbot provides a simple token bucket rate limiting framework used across all exchange modules.

## Design

`RateLimiter` in `jackbot-integration` controls the maximum number of operations allowed within a time window. It supports three priority queues (`High`, `Normal`, and `Low`) and adaptive backoff with optional jitter when a violation is reported. Logging via the `tracing` crate is emitted when limits are reached or backoff is triggered.

Each exchange exposes a `*RateLimit` struct wrapping two `RateLimiter` instances – one for REST requests and one for WebSocket messages. Constructors provide sensible default quotas and a `with_params` helper for tests.

Example usage:

```rust
let limits = BinanceRateLimit::new();
limits.acquire_rest(Priority::High).await;
```

## Current Status

Rate limit modules are implemented for:

- Binance
- Bitget
- Bybit
- Coinbase
- Hyperliquid
- Kraken
- OKX
- Kucoin

Other exchanges will be added as integrations mature.
