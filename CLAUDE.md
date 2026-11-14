# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Trading Bot (`trading-bot-*` crates, repo `trapilot/rust-trading-bot`) is a Cargo workspace (edition 2024, resolver 3, stable toolchain via `rust-toolchain.toml`) of Rust libraries for live-trading, paper-trading and back-testing systems. Library crates live under `crates/<short-name>/` (runtime, market, execution, instrument, integration, macro) with package names `trading-bot-<short-name>`. Runnable programs live in `apps/` (package `trading-bot-apps`, `publish = false`): one binary per file in `apps/src/bin/`, the criterion benches in `apps/benches/`, plus their JSON inputs in `apps/config/` and `apps/data/`. These paths are relative to the repo root, so run the apps from there. The code is a renamed fork of barter-rs. Don't remove the original copyright lines from the LICENSE files, because MIT requires them. release-plz writes a per-crate `CHANGELOG.md`.

## Commands

CI (`.github/workflows/ci.yml`) runs these on the whole workspace. A change must pass all four:

```bash
cargo check
cargo test                       # CI sets CARGO_PROFILE_DEV_DEBUG=0 (large binaries overflow DWARF32 under rust-lld)
cargo fmt --all -- --check
cargo clippy -- -D warnings      # workspace is kept warning-free
```

Narrower runs:

```bash
cargo test -p trading-bot-market                          # one crate
cargo test -p trading-bot-runtime <test_name_substring>         # one test by name
cargo test -p trading-bot-runtime --test test_engine_process_engine_event_with_audit   # one integration test file
cargo run -p trading-bot-apps --bin engine_sync_with_live_market_data_and_mock_execution_and_audit   # from repo root
cargo bench -p trading-bot-apps --bench backtest                # criterion benches in apps/benches/backtest
```

`trading-bot-integration` gates most modules behind features (`collection`, `channel`, `error`, `protocol`, `serde`, `subscription`, `socket`, `stream`, `metric`; `full` enables all of them, and `channel` is **not** in `default`). When you change gated code, also check the feature combination it builds under, e.g. `cargo check -p trading-bot-integration --no-default-features --features stream`.

`rustfmt.toml` sets `imports_granularity = "crate"`, so `use` statements are merged per crate.

## Crate dependency graph

```
trading-bot-macro        (proc-macros, used by trading-bot-market)
trading-bot-integration  (low-level REST/WebSocket framework: RestClient, Signer, StreamParser,
                         ReconnectingSocket, TradingBotStreamExt, Validator/Transformer, collections)
trading-bot-instrument   (Exchange/Asset/Instrument types + IndexedInstruments)
trading-bot-market       → integration, instrument, macro   (public market data streams)
trading-bot-execution    → integration, instrument          (private account data + order execution)
trading-bot-runtime      → all of the above                 (Engine, System, strategy, risk, statistics, backtest)
trading-bot-apps         → runtime + all of the above       (runnable binaries, not published)
```

Internal crates are referenced via `[workspace.dependencies]` in the root `Cargo.toml`, with pinned versions. A breaking change in a lower crate cascades into version bumps of the crates above it.

## Architecture

### Indexed data model (trading-bot-instrument)
`IndexedInstruments` (`crates/instrument/src/index`) turns configured instruments into dense `ExchangeIndex` / `AssetIndex` / `InstrumentIndex` keys. Most generic types across the workspace take `ExchangeKey`, `AssetKey` and `InstrumentKey` type parameters that default to these indexes. Engine state is stored in index-addressed collections, so lookups are O(1). Market streams and account streams are "indexed" (keys mapped from exchange names to indexes) before they reach the Engine.

### trading-bot-market
- `exchange/<venue>/` implements `Connector` (plus `StreamSelector<Instrument, Kind>`) for each venue: binance (spot/futures), bitfinex, bitmex, bybit, coinbase, gateio, kraken, okx.
- `subscription/` holds the `SubscriptionKind`s (PublicTrades, OrderBooksL1/L2, Liquidations, …). `subscriber/` maps and validates subscriptions over the WebSocket. `transformer/` (`ExchangeTransformer`) turns venue messages into normalised `MarketEvent`s.
- `streams/builder` builds single-kind and dynamic multi-exchange streams. `streams/reconnect` wraps them in auto-reconnecting `MarketStreamEvent` streams (`Item` / `Reconnecting`).
- `books/` holds the L2 order book manager.

### trading-bot-execution
- `ExecutionClient` trait (`client/mod.rs`): a venue integration for account snapshots, the account event stream, open/cancel orders, balances and trades. Implementations: `client/binance`, `client/mock`.
- `exchange/mock` is a simulated exchange that the mock client talks to over channels. It powers paper trading and backtests.
- `indexer.rs` maps account events onto the indexed key space.

### trading-bot-runtime
- `Engine<Clock, State, ExecutionTxs, Strategy, Risk>` (`engine/mod.rs`) implements `Processor<EngineEvent>`. It updates `EngineState`, and if `TradingState::Enabled` it generates algo orders → `RiskManager` checks → sends requests through `ExecutionTxMap`. Every `process` call produces an `EngineAudit`, which `Auditor` wraps into an `AuditTick` with a sequence number.
- `EngineEvent` (`lib.rs`) has five variants: `Shutdown`, `Command`, `TradingStateUpdate`, `Account(..)`, `Market(..)`. Custom event types are supported by implementing `Processor`.
- `engine/state/`: `EngineState<GlobalData, InstrumentData>` made up of connectivity, asset, instrument (incl. user-defined `InstrumentDataState`), order (`OrderManager`, in-flight recording), position and trading state.
- `engine/action/`: the actions behind `Command`s (send requests, cancel orders, close positions, generate algo orders).
- `engine/run.rs`: the runners `sync_run[_with_audit]` (Iterator feed) and `async_run[_with_audit]` (Stream feed).
- `engine/audit/`: the audit stream, plus `StateReplicaManager`, which keeps a non-hot-path `EngineState` replica in sync from that stream.
- `engine/clock.rs`: `EngineClock`. Use `LiveClock` for live trading, or `HistoricalClock`, which derives time from events, for backtests.
- `strategy/`: strategy is split into separate traits: `AlgoStrategy`, `ClosePositionsStrategy`, `OnDisconnectStrategy`, `OnTradingDisabled`. `DefaultStrategy` implements all of them.
- `risk/`: `RiskManager` plus reusable `RiskCheck`s. `DefaultRiskManager` approves everything.
- `execution/`: `ExecutionManager`s, one per exchange, that sit between the Engine and an `ExecutionClient`.
- `system/`: `SystemBuilder` + `SystemArgs` + `SystemConfig` (JSON, see `apps/config/`). It wires together market streams, execution managers, the Engine task (`EngineFeedMode::Iterator` vs `Stream`), audit mode and the initial trading state. A `System` handle exposes `trading_state`, `cancel_orders`, `close_positions` and `shutdown`.
- `statistic/` + `backtest/`: trading summaries / tear sheets (PnL, Sharpe, Sortino, drawdown) and utilities for running many concurrent backtests over historic market data (`apps/data/`).

Backtests use the same Engine as live trading. The only differences are a historic market data stream, the mock execution client/exchange, and a historical clock.

## Conventions

- Commits follow Conventional Commits with crate/area scopes, e.g. `fix(gateio)!: ...`, `perf(integration): ...`, `refactor(integration/http): ...`. Mark breaking changes with `!`. Releases and changelogs are generated by release-plz (`release-plz.toml`) from these messages.
- PRs target `main`.
