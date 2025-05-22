# Exchange Feature Implementation Status

**Supported Exchanges**
* Binance
* Bitget
* Bybit
* Coinbase
* Hyperliquid
* Kraken
* MEXC
* Kucoin
* Gate.io
* Crypto.com
* OKX

**Instructions for Contributors:**
- Check off each box as you complete the step for each task.
- If an exchange/market does not support intended functionality, document it here and in the code with comments.

**Conventions:**
- All public types and functions must be documented.
- Each exchange must have a test module or file for each feature (unit or integration tests).
- If a feature is not supported, a stub file with a doc comment must be present explaining why.

**📈 Progress Tracking**
- Use checkboxes above to track status
- Tag PRs/issues with relevant exchange and feature (e.g., `[bybit][orderbook-l2]`)
- Assign owners for each major task
- Update this file on every merge

**📝 Update Guidelines**
- All contributors **must** update this file with each significant feature, bugfix, or doc change
- Add new rows for new exchanges or features as needed
- If a feature is not supported by an exchange, explicitly document it here
- Keep the feature matrix and TODOs current for team visibility

## Canonical Order Book Representation

We've implemented a new framework for standardizing all exchange-specific orderbook formats into a canonical representation:

- **New file:** `jackbot-data/src/books/canonical.rs` defines a `Canonicalizer` trait and a `CanonicalOrderBook` wrapper 
- **Purpose:** This replaces the previous "normalization" terminology (which could be confused with ML normalization techniques)
- **Benefits:** 
  - Consistent interface across all exchanges
  - Standardized conversion of exchange-specific formats
  - Additional utility methods for orderbook analysis (mid price, spread calculations, etc.)
  - Clear separation between exchange-specific formats and our internal representation

Exchanges currently implementing the `Canonicalizer` trait:
- Bybit (Spot & Futures)
- Kraken (Spot & Futures)
- Binance (Spot & Futures)
- OKX (Spot & Futures)
- Coinbase (Spot)

## 🚧 TODO: L2 Order Book (Spot & Futures) Implementation Plan

> **Goal:** Implement robust, fully-tested L2 order book (WebSocket) support for both spot and futures for all project exchanges, following the new folder/module conventions and leveraging the `L2Sequencer` abstraction where possible.

**General Steps (repeat for each exchange and market type):**
- [x] Research and document at `docs/L2_DATA_STREAMS.md` latest L2 order book WS API for spot/futures for all supported exchanges.
- [ ] Scaffold or refactor `spot/l2.rs` and `futures/l2.rs` (and `mod.rs`).
- [ ] Implement L2 order book logic: subscribe, parse, normalize, maintain local book, handle sequencing/resync.
- [ ] Add/extend unit and integration tests (including edge cases).
- [ ] Add/extend module-level docs.
-

**Exchange-Specific TODOs:**

- **Binance**
  - [x] Refactor `spot/l2.rs` to use new `L2Sequencer` trait. (Complete, tested)
  - [x] Refactor `futures/l2.rs` to use new `L2Sequencer` trait. (Complete, tested)
  - [x] Expand tests for both. (L2 implementation robust and fully tested)
  - [x] Update to use new `Canonicalizer` trait.

- **Bitget**
  - [x] Implement `spot/l2.rs` (L2 order book, WS, incremental).
  - [x] Implement `futures/l2.rs` (L2 order book, WS, incremental).
  - [x] Add/extend tests for both.
  - [x] Update to use new `Canonicalizer` trait.

- **Bybit**
  - [x] Implement/refactor `spot/l2.rs` (L2 order book, WS, incremental). (Complete and fixed error handling)
  - [x] Implement/refactor `futures/l2.rs` (L2 order book, WS, incremental). (Complete and fixed error handling)
  - [x] Update to use new `Canonicalizer` trait. (Complete for both spot and futures)
  - [x] Add/extend tests for both. (Sequencing, snapshots, canonicalization)

- **Coinbase**
  - [x] Implement/refactor `spot/l2.rs` (L2 order book, WS, incremental). (Complete and tested)
  - [N/A] Implement/refactor `futures/l2.rs` (L2 order book, WS, incremental). (Coinbase doesn't support futures trading)
  - [x] Add/extend tests for both. (Tests for spot included)
  - [x] Update to use new `Canonicalizer` trait.

- **Kraken**
  - [x] Implement/refactor `spot/l2.rs` (L2 order book, WS, incremental). (Complete with tests)
  - [x] Implement/refactor `futures/l2.rs` (L2 order book, WS, incremental). (Complete with snapshot support, transformer, and canonicalization)
  - [x] Add/extend tests for both.
- [x] Update futures to use new `Canonicalizer` trait. (Complete)
  - [x] Update spot to use new `Canonicalizer` trait.

- **Kucoin**
  - [ ] Implement/refactor `spot/l2.rs` (L2 order book, WS, incremental). (Partially implemented, needs testing)
  - [ ] Implement/refactor `futures/l2.rs` (L2 order book, WS, incremental). (Not yet implemented)
  - [ ] Add/extend tests for both.
  - [ ] Update to use new `Canonicalizer` trait.

- **OKX**
  - [x] Implement/refactor `spot/l2.rs` (L2 order book, WS, incremental). (Complete with snapshot support and tests)
  - [x] Implement/refactor `futures/l2.rs` (L2 order book, WS, incremental). (Complete with snapshot support and tests)
  - [x] Add/extend tests for both.
  - [x] Update to use new `Canonicalizer` trait.

- **Hyperliquid**
  - [ ] Implement/refactor `spot/l2.rs` (L2 order book, WS, incremental).
  - [ ] Implement/refactor `futures/l2.rs` (L2 order book, WS, incremental).
  - [ ] Add/extend tests for both.
  - [ ] Update to use new `Canonicalizer` trait.

- **MEXC**
  - [ ] Implement/refactor `spot/l2.rs` (L2 order book, WS, incremental).
  - [ ] Implement/refactor `futures/l2.rs` (L2 order book, WS, incremental).
  - [ ] Add/extend tests for both.
  - [ ] Update to use new `Canonicalizer` trait.

- **Gate.io**
  - [ ] Implement/refactor `spot/l2.rs` (L2 order book, WS, incremental).
  - [ ] Implement/refactor `futures/l2.rs` (L2 order book, WS, incremental).
  - [ ] Add/extend tests for both.
  - [ ] Update to use new `Canonicalizer` trait.

- **Crypto.com**
  - [ ] Implement/refactor `spot/l2.rs` (L2 order book, WS, incremental).
  - [ ] Implement/refactor `futures/l2.rs` (L2 order book, WS, incremental).
  - [ ] Add/extend tests for both.
  - [ ] Update to use new `Canonicalizer` trait.

**Final Steps:**
- [x] Update feature matrix and exchange-by-exchange status in this file.
- [ ] Ensure all tests pass for all exchanges after each change.
- [ ] Document any API quirks, limitations, or unsupported features.

**Implementation Summary:**
- Complete L2 Order Book implementations for: Binance (Spot & Futures), Bybit (Spot & Futures), Coinbase (Spot), Kraken (Spot & Futures), OKX (Spot & Futures), Bitget (Spot & Futures)
- Partially implemented for: Kucoin (Spot), Crypto.com (Spot & Futures), MEXC (Spot & Futures), Hyperliquid (Spot & Futures)
- Not yet implemented for: Kucoin (Futures), Gate.io
- Canonicalizer implementations for: Bybit (Spot & Futures), Kraken (Spot & Futures), Binance (Spot & Futures), OKX (Spot & Futures), Coinbase (Spot), Bitget (Spot & Futures), MEXC (Spot & Futures), Crypto.com (Spot & Futures), Hyperliquid (Spot & Futures)

**Next Steps:**
1. Complete existing implementations with robust testing
2. Implement for remaining exchanges
3. Ensure proper snapshot support and sequencing for all exchanges
4. Add comprehensive error handling and recovery for WebSocket disruptions
5. Update all implementations to use the new `Canonicalizer` trait

## 🚧 TODO: Trades WebSocket Listener Implementation Plan

> **Goal:** Implement robust, fully-tested trade WebSocket listeners for both spot and futures for all project exchanges, following the new folder/module conventions and ensuring normalized trade event handling.

**General Steps (repeat for each exchange and market type):**
- [ ] Research and document latest trade WS API for spot/futures.
- [ ] Scaffold or refactor `spot/trade.rs` and `futures/trade.rs` (and `mod.rs`).
- [ ] Implement trade WebSocket subscription logic: subscribe, parse, normalize, and emit trade events.
- [ ] Add/extend unit and integration tests (including edge cases).
- [ ] Add/extend module-level docs.
- [ ] Update `docs/IMPLEMENTATION_STATUS.md` with status and links.

**Exchange-Specific TODOs:**

- **Binance**
  - [x] Implement/refactor `spot/trade.rs` (trade WS listener).
  - [x] Implement/refactor `futures/trade.rs` (trade WS listener).
  - [x] Add/extend tests for both.

- **Bitget**
  - [ ] Implement/refactor `spot/trade.rs` (trade WS listener).
  - [ ] Implement/refactor `futures/trade.rs` (trade WS listener).
  - [ ] Add/extend tests for both.

- **Bybit**
  - [ ] Implement/refactor `spot/trade.rs` (trade WS listener).
  - [ ] Implement/refactor `futures/trade.rs` (trade WS listener).
  - [ ] Add/extend tests for both.

- **Coinbase**
  - [ ] Implement/refactor `spot/trade.rs` (trade WS listener).
  - [ ] Implement/refactor `futures/trade.rs` (trade WS listener).
  - [ ] Add/extend tests for both.

- **Kraken**
  - [ ] Implement/refactor `spot/trade.rs` (trade WS listener).
  - [ ] Implement/refactor `futures/trade.rs` (trade WS listener).
  - [ ] Add/extend tests for both.

- **Kucoin**
  - [ ] Implement/refactor `spot/trade.rs` (trade WS listener).
  - [ ] Implement/refactor `futures/trade.rs` (trade WS listener).
  - [ ] Add/extend tests for both.

- **OKX**
  - [ ] Implement/refactor `spot/trade.rs` (trade WS listener).
  - [ ] Implement/refactor `futures/trade.rs` (trade WS listener).
  - [ ] Add/extend tests for both.

- **Hyperliquid**
  - [ ] Implement/refactor `spot/trade.rs` (trade WS listener).
  - [ ] Implement/refactor `futures/trade.rs` (trade WS listener).
  - [ ] Add/extend tests for both.

- **MEXC**
  - [ ] Implement/refactor `spot/trade.rs` (trade WS listener).
  - [ ] Implement/refactor `futures/trade.rs` (trade WS listener).
  - [ ] Add/extend tests for both.

- **Gate.io**
  - [ ] Implement/refactor `spot/trade.rs` (trade WS listener).
  - [ ] Implement/refactor `futures/trade.rs` (trade WS listener).
  - [ ] Add/extend tests for both.

- **Crypto.com**
  - [ ] Implement/refactor `spot/trade.rs` (trade WS listener).
  - [ ] Implement/refactor `futures/trade.rs` (trade WS listener).
  - [ ] Add/extend tests for both.

**Final Steps:**
- [ ] Update feature matrix and exchange-by-exchange status in this file.
- [ ] Ensure all tests pass for all exchanges after each change.
- [ ] Document any API quirks, limitations, or unsupported features.

**This matrix and TODO list must be kept up to date by all contributors.**

## L2 Order Book Sequencer Abstraction (In Progress)

- **New file:** `jackbot-data/src/books/l2_sequencer.rs` defines a generic `L2Sequencer` trait and a `HasUpdateIds` trait for L2 update types.
- **Binance Spot:** Sequencer logic is being migrated to implement the new trait.
- **Binance Futures:** Will be migrated next.
- **Goal:** Remove duplicated sequencing logic and standardize L2 order book update handling across exchanges.

## Next Steps
- Refactor `binance/spot/l2.rs` and `binance/futures/l2.rs` to use the new trait.
- Implement `HasUpdateIds` for their update types.
- Expand tests to cover the new abstraction.

## Other Exchanges
- OKX, Bybit, Kraken, etc. do not currently require sequencing logic, but can opt-in to the new trait if needed in the future.

## Recent Changes

- All L1 (Level 1) order book code, modules, and examples have been **removed** from the project.
- Only L2 (Level 2) streams are now supported.
- All L1 types, subscription kinds, and references have been deleted from the codebase.
- Example files dedicated to L1 streams have also been removed.

## Current Features

- L2 order book streams for all supported exchanges.
- No L1 order book support (L2 streams contain L1 data).

## Where to Find Things

- L2 stream implementations: `jackbot-data/src/exchange/*/l2.rs`
- Subscription kinds: `jackbot-data/src/subscription/book.rs` (L2 only)

## What is Missing

- No L1 order book support (by design).

---

**This file is up to date as of the L1 code removal migration.**

## 🚧 TODO: jackbot-execution Live & Paper Trading Support

> **Goal:** Refactor and extend `jackbot-execution` to support both live and paper trading on all supported exchanges (spot and futures), with robust abstraction, error handling, and test coverage.

**General Steps:**
- [ ] Research and document trading (order management) APIs for all supported exchanges (spot/futures).
- [ ] Design/extend a unified trading abstraction (trait/interface) for order placement, cancellation, modification, and status queries.
- [ ] Implement or refactor exchange adapters for live trading (real orders via authenticated API/WebSocket).
- [ ] Implement a robust paper trading engine (simulated fills, order book emulation, event emission, etc.).
- [ ] Add/extend integration tests for both live and paper trading (with mocks/sandboxes where possible).
- [ ] Add/extend module-level and user-facing documentation.
- [ ] Update `docs/IMPLEMENTATION_STATUS.md` with status and links.

**Exchange-Specific TODOs:**

- **Binance**
  - [ ] Implement/refactor live trading adapter (spot/futures).
  - [ ] Implement/refactor paper trading adapter (spot/futures).
  - [ ] Add/extend tests for both.

- **Bitget**
  - [ ] Implement/refactor live trading adapter (spot/futures).
  - [ ] Implement/refactor paper trading adapter (spot/futures).
  - [ ] Add/extend tests for both.

- **Bybit**
  - [ ] Implement/refactor live trading adapter (spot/futures).
  - [ ] Implement/refactor paper trading adapter (spot/futures).
  - [ ] Add/extend tests for both.

- **Coinbase**
  - [ ] Implement/refactor live trading adapter (spot/futures).
  - [ ] Implement/refactor paper trading adapter (spot/futures).
  - [ ] Add/extend tests for both.

- **Kraken**
  - [ ] Implement/refactor live trading adapter (spot/futures).
  - [ ] Implement/refactor paper trading adapter (spot/futures).
  - [ ] Add/extend tests for both.

- **Kucoin**
  - [ ] Implement/refactor live trading adapter (spot/futures).
  - [ ] Implement/refactor paper trading adapter (spot/futures).
  - [ ] Add/extend tests for both.

- **OKX**
  - [ ] Implement/refactor live trading adapter (spot/futures).
  - [ ] Implement/refactor paper trading adapter (spot/futures).
  - [ ] Add/extend tests for both.

- **Hyperliquid**
  - [ ] Implement/refactor live trading adapter (spot/futures).
  - [ ] Implement/refactor paper trading adapter (spot/futures).
  - [ ] Add/extend tests for both.

- **MEXC**
  - [ ] Implement/refactor live trading adapter (spot/futures).
  - [ ] Implement/refactor paper trading adapter (spot/futures).
  - [ ] Add/extend tests for both.

- **Gate.io**
  - [ ] Implement/refactor live trading adapter (spot/futures).
  - [ ] Implement/refactor paper trading adapter (spot/futures).
  - [ ] Add/extend tests for both.

- **Crypto.com**
  - [ ] Implement/refactor live trading adapter (spot/futures).
  - [ ] Implement/refactor paper trading adapter (spot/futures).
  - [ ] Add/extend tests for both.

**Final Steps:**
- [ ] Update feature matrix and exchange-by-exchange status in this file.
- [ ] Ensure all tests pass for all exchanges after each change.
- [ ] Document any API quirks, limitations, or unsupported features.

---

**Instructions for Contributors:**
- Check off each box as you complete the step for each exchange/market.
- Link to PRs/issues and relevant code in the status section.
- If an exchange/market does not support trading, document it here and in the code as a stub.

## 🚧 TODO: Smart Trades (Advanced Order Types)

> **Goal:** Implement advanced smart trade features for all supported exchanges and both live/paper trading: trailing take profit, profit at predetermined price levels, trailing stop loss, and multi-level stop loss. Ensure robust abstraction, event handling, and test coverage.

**General Steps:**
- [ ] Research and document advanced order type support and limitations for all supported exchanges (spot/futures).
- [ ] Design/extend a unified abstraction for smart trade strategies (modular, composable, and testable).
- [ ] Implement trailing take profit logic (dynamic adjustment as price moves in favor).
- [ ] Implement profit at predetermined price levels (partial or full closes at set targets).
- [ ] Implement trailing stop loss logic (dynamic stop that follows price).
- [ ] Implement multi-level stop loss (multiple stop levels, e.g., stepwise risk reduction).
- [ ] Integrate with both live and paper trading engines.
- [ ] Add/extend integration and unit tests for all smart trade features (including edge cases and race conditions).
- [ ] Add/extend module-level and user-facing documentation.
- [ ] Update `docs/IMPLEMENTATION_STATUS.md` with status and links.

**Feature-Specific TODOs:**

- [ ] Trailing Take Profit (all exchanges, spot/futures, live/paper)
- [ ] Profit at Predetermined Price Levels (all exchanges, spot/futures, live/paper)
- [ ] Trailing Stop Loss (all exchanges, spot/futures, live/paper)
- [ ] Multi-Level Stop Loss (all exchanges, spot/futures, live/paper)
- [ ] MEXC: Implement all smart trade features (spot/futures, live/paper)
- [ ] Gate.io: Implement all smart trade features (spot/futures, live/paper)
- [ ] Crypto.com: Implement all smart trade features (spot/futures, live/paper)

**Final Steps:**
- [ ] Update feature matrix and exchange-by-exchange status in this file.
- [ ] Ensure all tests pass for all exchanges after each change.
- [ ] Document any API quirks, limitations, or unsupported features.

---

**Instructions for Contributors:**
- Check off each box as you complete the step for each feature/exchange/market.
- Link to PRs/issues and relevant code in the status section.
- If an exchange/market does not support a smart trade feature, document it here and in the code as a stub.

## 🚧 TODO: Advanced Execution Order Types (Always Maker, TWAP, VWAP)

> **Goal:** Implement advanced execution order types for all supported exchanges and both live/paper trading: 'always maker' (post-only, top-of-book, auto-cancel/repost), and advanced TWAP/VWAP with untraceable curves using order book blending and jackbot-data analytics. Ensure robust abstraction, event handling, and test coverage.

**General Steps:**
- [ ] Research and document post-only/maker order support and limitations for all supported exchanges (spot/futures).
- [ ] Design/extend a unified abstraction for advanced execution strategies (modular, composable, and testable).
- [ ] Implement 'always maker' order logic:
    - [ ] Place post-only order at top of book (best bid for buy, best ask for sell).
    - [ ] Auto-cancel after 3 seconds if not filled, and repost at new top of book.
    - [ ] Repeat until filled or user cancels.
    - [ ] Ensure lowest (maker) fees and fast fills.
- [x] Implement advanced TWAP (Time-Weighted Average Price) logic:
    - [x] Split order into slices over time.
    - [x] Use untraceable, non-linear time curves and randomized intervals.
    - [x] Blend with observed order book behavior from jackbot-data to avoid detection.
- [x] Implement advanced VWAP (Volume-Weighted Average Price) logic:
    - [x] Split order based on observed volume patterns.
    - [x] Use untraceable, non-linear volume curves and randomized intervals.
    - [x] Blend with order book and trade flow analytics from jackbot-data.
- [x] Integrate with both live and paper trading engines.
- [x] Add/extend integration and unit tests for all advanced order types (including edge cases and race conditions).
- [x] Add/extend module-level and user-facing documentation.
- [x] Update `docs/IMPLEMENTATION_STATUS.md` with status and links.

**Feature-Specific TODOs:**

 - [x] Always Maker (post-only, top-of-book, auto-cancel/repost, all exchanges, spot/futures, live/paper)
- [x] Advanced TWAP (untraceable, order book blended, all exchanges, spot/futures, live/paper)
- [x] Advanced VWAP (untraceable, order book blended, all exchanges, spot/futures, live/paper)
- [ ] MEXC: Implement all advanced execution order types (spot/futures, live/paper)
- [ ] Gate.io: Implement all advanced execution order types (spot/futures, live/paper)
- [ ] Crypto.com: Implement all advanced execution order types (spot/futures, live/paper)

**Final Steps:**
- [ ] Update feature matrix and exchange-by-exchange status in this file.
- [ ] Ensure all tests pass for all exchanges after each change.
- [ ] Document any API quirks, limitations, or unsupported features.

---

**Instructions for Contributors:**
- Check off each box as you complete the step for each feature/exchange/market.
- Link to PRs/issues and relevant code in the status section.
- If an exchange/market does not support an advanced order type, document it here and in the code as a stub.

## 🚧 TODO: Prophetic Orders (Out-of-Book Limit Order Capture & Placement)

> **Goal:** Implement 'prophetic orders' for all supported exchanges and both live/paper trading: allow users to specify limit orders far outside the allowed order book range, track these in jackbot, and automatically place them the instant the order book comes in range. Include robust range detection, event handling, and test coverage.

**General Steps:**
- [ ] Research and document order book price range enforcement for all supported exchanges (spot/futures).
- [x] Design/extend a unified abstraction for prophetic order management (modular, composable, and testable).
- [x] Implement logic to:
    - [x] Accept and store user prophetic orders (way out of book) in jackbot.
    - [x] Monitor real-time order book for each symbol.
    - [x] Detect when the order book comes in range to accept the limit order.
    - [x] Instantly place the order on the exchange when in range.
    - [ ] Handle edge cases (race conditions, rapid book moves, partial fills, cancellations).
- [x] Implement tests to empirically determine the real price range supported by each exchange (spot/futures):
    - [x] Place test orders at various distances from the market.
    - [x] Record and document the actual allowed range for each exchange/market.
    - [x] Automate this as part of the test suite.
- [ ] Integrate with both live and paper trading engines.
- [ ] Add/extend integration and unit tests for all prophetic order logic (including edge cases and race conditions).
- [ ] Add/extend module-level and user-facing documentation.
- [ ] Update `docs/IMPLEMENTATION_STATUS.md` with status and links.

**Feature-Specific TODOs:**

 - [x] Prophetic Orders (capture, monitor, auto-place, all exchanges, spot/futures, live/paper)
 - [x] Exchange Range Detection (empirical, automated, all exchanges, spot/futures)
- [ ] MEXC: Implement all prophetic order logic and range detection (spot/futures, live/paper)
- [ ] Gate.io: Implement all prophetic order logic and range detection (spot/futures, live/paper)
- [ ] Crypto.com: Implement all prophetic order logic and range detection (spot/futures, live/paper)

**Final Steps:**
- [ ] Update feature matrix and exchange-by-exchange status in this file.
- [ ] Ensure all tests pass for all exchanges after each change.
- [ ] Document any API quirks, limitations, or unsupported features.

## 🚧 TODO: Jackpot Orders (High Leverage Bets with Controlled Loss)

**Status:** Initial support for isolated high-leverage orders with strict loss limits has been implemented. Positions are monitored and will auto-close when the ticket loss threshold is reached across exchanges.

> **Goal:** Implement 'jackpot orders' for all supported exchanges and both live/paper trading: allow users to place high leverage (e.g., x100, x200) long or short bets with strictly controlled loss (ticket size), using isolated margin high leverage perpetual orders. Ensure robust abstraction, risk management, event handling, and test coverage.

**General Steps:**
- [ ] Research and document isolated margin and high leverage perpetual order support for all supported exchanges (spot/futures).
- [ ] Design/extend a unified abstraction for jackpot order management (modular, composable, and testable).
- [ ] Implement logic to:
    - [x] Allow users to specify leverage (e.g., x100, x200), direction (long/short), and ticket size (max loss).
    - [x] Place isolated margin high leverage perpetual orders (long or short) on supported exchanges.
    - [x] Monitor position and enforce strict loss control (auto-close/liquidate at ticket loss threshold).
    - [ ] Handle edge cases (exchange liquidation, margin calls, slippage, rapid price moves).
    - [ ] Provide clear user feedback and risk warnings.
- [ ] Integrate with both live and paper trading engines.
- [ ] Add/extend integration and unit tests for all jackpot order logic (including edge cases and race conditions).
- [ ] Add/extend module-level and user-facing documentation.
- [ ] Update `docs/IMPLEMENTATION_STATUS.md` with status and links.

**Feature-Specific TODOs:**

- [x] Jackpot Orders (high leverage, controlled loss, all exchanges, futures/perpetuals, live/paper)
- [x] Risk Control & Monitoring (auto-close, ticket enforcement, all exchanges)
- [ ] MEXC: Implement all jackpot order logic and risk control (futures/perpetuals, live/paper)
- [ ] Gate.io: Implement all jackpot order logic and risk control (futures/perpetuals, live/paper)
- [ ] Crypto.com: Implement all jackpot order logic and risk control (futures/perpetuals, live/paper)

**Final Steps:**
- [ ] Update feature matrix and exchange-by-exchange status in this file.
- [ ] Ensure all tests pass for all exchanges after each change.
- [ ] Document any API quirks, limitations, or unsupported features.

---

**Instructions for Contributors:**
- Check off each box as you complete the step for each task.
- If an exchange/market does not support jackpot orders, document it here and in the code with comments.

## 🚧 TODO: Redis Order Book & Trade Data Representation

> **Goal:** Implement a Redis-backed, real-time representation of all order book and trade data being fetched for all supported exchanges and markets. Ensure efficient, consistent, and scalable storage and retrieval for downstream consumers and analytics.

**General Steps:**
- [x] Design a Redis schema for storing order book snapshots, deltas, and trade events (multi-exchange, multi-market).
- [x] Implement efficient serialization/deserialization for order book and trade data (e.g., JSON, MessagePack, or binary).
- [x] Integrate Redis updates into the order book and trade WebSocket handlers for all exchanges/markets.
- [x] Ensure atomicity and consistency of updates (e.g., use Redis transactions or Lua scripts for multi-key updates).
- Snapshot keys use the pattern `jb:<exchange>:<instrument>:snapshot`.
- Delta lists use `jb:<exchange>:<instrument>:deltas` and trades are stored under `jb:<exchange>:<instrument>:trades`.
- All writes are performed via Redis pipelines with `.atomic()` to guarantee consistency.
- [ ] Implement efficient querying and subscription mechanisms for downstream consumers (e.g., pub/sub, streams, sorted sets).
- [ ] Add/extend integration and unit tests for Redis logic (including edge cases, reconnections, and data consistency).
- [ ] Add/extend module-level and user-facing documentation.
- [ ] Update `docs/IMPLEMENTATION_STATUS.md` with status and links.

**Feature-Specific TODOs:**

- [ ] Redis Order Book Storage (all exchanges, spot/futures)
- [ ] Redis Trade Data Storage (all exchanges, spot/futures)
- [ ] Multi-Exchange/Market Keying & Namespacing
- [ ] Efficient Delta/Update Handling
- [ ] Downstream Consumer API (pub/sub, streams, etc.)
- [x] MEXC: Integrate Redis for order book and trades
- [ ] Gate.io: Integrate Redis for order book and trades
- [ ] Crypto.com: Integrate Redis for order book and trades

**Final Steps:**
- [ ] Update feature matrix and exchange-by-exchange status in this file.
- [ ] Ensure all tests pass for all exchanges after each change.
- [ ] Document any API quirks, limitations, or unsupported features.

---

**Instructions for Contributors:**
- Check off each box as you complete the step for each task.
- If an exchange/market does not support Redis integration, document it here and in the code with comments.

## 🚧 TODO: Redis Snapshot to S3 (Parquet + Iceberg Data Lake)

> **Goal:** Implement a mechanism to periodically save snapshots of cached order book and trade data from Redis to S3 in Parquet format, using Apache Iceberg for data lake management. Ensure scalable, queryable, and cost-efficient historical data storage for analytics and research.

**General Steps:**
- [x] Design a snapshot schema for order book and trade data (columnar, analytics-friendly).
- [x] Implement efficient extraction of data from Redis (batch, streaming, or point-in-time snapshot).
- [x] Serialize and write data to Parquet format (using appropriate libraries for Rust or via ETL pipeline).
- [x] Integrate with S3 for scalable, reliable storage (handle credentials, retries, partitioning).
- [x] Register and manage Parquet files with Apache Iceberg for data lake organization and queryability.
- [x] Implement snapshot scheduling (periodic, on-demand, or event-driven).
- [x] Add/extend integration and unit tests for snapshot, S3, and Iceberg logic (including edge cases, failures, and recovery).
- [x] Add/extend module-level and user-facing documentation.
- [x] Update `docs/IMPLEMENTATION_STATUS.md` with status and links.

**Feature-Specific TODOs:**

- [x] Parquet Serialization (order book, trades, multi-exchange/market)
- [x] S3 Integration (upload, partitioning, retention)
- [x] Iceberg Table Management (registration, schema evolution, query support)
- [x] Snapshot Scheduling (configurable, robust)
- [ ] MEXC: Integrate snapshot logic for order book and trades
- [ ] Gate.io: Integrate snapshot logic for order book and trades
- [ ] Crypto.com: Integrate snapshot logic for order book and trades

**Final Steps:**
- [ ] Update feature matrix and exchange-by-exchange status in this file.
- [ ] Ensure all tests pass for all exchanges after each change.
- [ ] Document any API quirks, limitations, or unsupported features.

---

**Instructions for Contributors:**
- Check off each box as you complete the step for each task.
- If an exchange/market does not support snapshotting, document it here and in the code with comments.

## 🚧 TODO: User WebSockets (Account Balance & Trading)

> **Goal:** Implement user WebSocket connections for all supported exchanges and markets to enable real-time account balance updates and trading (order events, fills, etc.). Ensure secure authentication, robust event handling, and unified abstraction for downstream consumers.

**General Steps:**
- [ ] Research and document user WebSocket API support and authentication mechanisms for all supported exchanges (spot/futures).
- [ ] Scaffold or refactor user WebSocket modules (e.g., `spot/user_ws.rs`, `futures/user_ws.rs`, and `mod.rs`).
- [ ] Implement secure authentication and connection management (API keys, signatures, session renewal, etc.).
- [ ] Implement event handlers for:
    - [ ] Account balance updates (deposits, withdrawals, transfers, PnL, margin changes).
    - [ ] Order events (new, filled, partially filled, canceled, rejected, etc.).
    - [ ] Position updates (for futures/perpetuals).
- [ ] Normalize and emit events for downstream consumers (internal APIs, Redis, etc.).
- [ ] Add/extend integration and unit tests for all user WebSocket logic (including edge cases, reconnections, and error handling).
- [ ] Add/extend module-level and user-facing documentation.
- [ ] Update `docs/IMPLEMENTATION_STATUS.md` with status and links.

**Exchange-Specific TODOs:**

- [ ] Binance: Implement/refactor user WebSocket for spot/futures (balance, trading events)
- [ ] Bitget: Implement/refactor user WebSocket for spot/futures (balance, trading events)
- [ ] Bybit: Implement/refactor user WebSocket for spot/futures (balance, trading events)
- [ ] Coinbase: Implement/refactor user WebSocket for spot/futures (balance, trading events)
- [ ] Kraken: Implement/refactor user WebSocket for spot/futures (balance, trading events)
- [ ] Kucoin: Implement/refactor user WebSocket for spot/futures (balance, trading events)
- [ ] OKX: Implement/refactor user WebSocket for spot/futures (balance, trading events)
- [ ] Hyperliquid: Implement/refactor user WebSocket for spot/futures (balance, trading events)
- [ ] MEXC: Implement/refactor user WebSocket for spot/futures (balance, trading events)
- [ ] Gate.io: Implement/refactor user WebSocket for spot/futures (balance, trading events)
- [ ] Crypto.com: Implement/refactor user WebSocket for spot/futures (balance, trading events)

**Final Steps:**
- [ ] Update feature matrix and exchange-by-exchange status in this file.
- [ ] Ensure all tests pass for all exchanges after each change.
- [ ] Document any API quirks, limitations, or unsupported features.

---

**Instructions for Contributors:**
- Check off each box as you complete the step for each task.
- If an exchange/market does not support user WebSockets, document it here and in the code with comments.

## 🚧 TODO: WebSocket Health Monitoring and Auto-Reconnect

> **Goal:** Implement robust health monitoring and auto-reconnection logic for all WebSocket connections across all exchanges and markets. Ensure consistent, reliable data flow with minimal downtime, intelligent backoff, and comprehensive logging.

**General Steps:**
- [ ] Design a unified health monitoring abstraction for WebSocket connections (heartbeats, pings, activity timeouts).
- [ ] Implement intelligent reconnection logic with exponential backoff and jitter for all exchanges/markets.
- [ ] Add monitoring metrics (uptime, latency, reconnect frequency, message throughput).
- [ ] Implement connection lifecycle events and error classification.
- [ ] Ensure proper handling of connection state during reconnection (subscription renewal, authentication refresh).
- [ ] Add resubscription logic for all data streams after reconnection.
- [ ] Implement circuit-breaker patterns for persistent failures.
- [ ] Add comprehensive logging and diagnostics for connection issues.
- [ ] Add/extend integration and unit tests for health monitoring and reconnection logic.
- [ ] Add/extend module-level and user-facing documentation.
- [ ] Update `docs/IMPLEMENTATION_STATUS.md` with status and links.

**Exchange-Specific TODOs:**

- [x] Binance: Implement/refactor health monitoring and reconnection for all WebSockets (spot/futures). (Heartbeat tracking, exponential backoff, and metrics added)
- [ ] Bitget: Implement/refactor health monitoring and reconnection for all WebSockets (spot/futures).
- [ ] Bybit: Implement/refactor health monitoring and reconnection for all WebSockets (spot/futures).
- [ ] Coinbase: Implement/refactor health monitoring and reconnection for all WebSockets (spot/futures).
- [ ] Kraken: Implement/refactor health monitoring and reconnection for all WebSockets (spot/futures).
- [ ] Kucoin: Implement/refactor health monitoring and reconnection for all WebSockets (spot/futures).
- [ ] OKX: Implement/refactor health monitoring and reconnection for all WebSockets (spot/futures).
- [ ] Hyperliquid: Implement/refactor health monitoring and reconnection for all WebSockets (spot/futures).
- [ ] MEXC: Implement/refactor health monitoring and reconnection for all WebSockets (spot/futures).
- [ ] Gate.io: Implement/refactor health monitoring and reconnection for all WebSockets (spot/futures).
- [ ] Crypto.com: Implement/refactor health monitoring and reconnection for all WebSockets (spot/futures).

**Final Steps:**
- [ ] Update feature matrix and exchange-by-exchange status in this file.
- [ ] Ensure all health monitoring and reconnection tests pass across all exchanges.
- [ ] Document any exchange-specific quirks, heartbeat patterns, or limitations.

---

## 🚧 TODO: API Rate Limiting and Backoff Strategies

> **Goal:** Implement comprehensive API rate limiting and intelligent backoff strategies for all REST and WebSocket API calls across all exchanges. Ensure compliance with exchange limits, prevent IP bans, and maintain service reliability under high load.

**General Steps:**
- [ ] Research and document rate limits for all supported exchanges (spot/futures, both REST and WebSocket).
- [ ] Design a unified rate limiting abstraction with per-endpoint, per-IP, and per-credential quotas.
- [ ] Implement adaptive backoff algorithms (exponential, with jitter) for rate limit violations.
- [ ] Add quota monitoring and enforcement for all API calls.
- [ ] Implement priority queueing for critical operations when approaching limits.
- [ ] Add rate limit remaining header parsing and adaptive quota adjustment.
- [ ] Implement circuit breakers for persistent rate limit violations.
- [ ] Add comprehensive logging and alerting for rate limit issues.
- [ ] Add/extend integration and unit tests for rate limiting and backoff logic.
- [ ] Add/extend module-level and user-facing documentation.
- [ ] Update `docs/IMPLEMENTATION_STATUS.md` with status and links.

**Exchange-Specific TODOs:**

- [ ] Binance: Implement/refactor rate limiting for REST/WebSocket (spot/futures).
- [ ] Bitget: Implement/refactor rate limiting for REST/WebSocket (spot/futures).
- [ ] Bybit: Implement/refactor rate limiting for REST/WebSocket (spot/futures).
- [ ] Coinbase: Implement/refactor rate limiting for REST/WebSocket (spot/futures).
- [ ] Kraken: Implement/refactor rate limiting for REST/WebSocket (spot/futures).
- [x] Kucoin: Rate limiting implemented for REST (30 req/3s) and WebSocket (100 msgs/10s) with adaptive jittered backoff.
- Kucoin REST quota: 30 requests/3s per IP. WebSocket quota: 100 messages/10s.
- [ ] OKX: Implement/refactor rate limiting for REST/WebSocket (spot/futures).
- [ ] Hyperliquid: Implement/refactor rate limiting for REST/WebSocket (spot/futures).
- [ ] MEXC: Implement/refactor rate limiting for REST/WebSocket (spot/futures).
- [ ] Gate.io: Implement/refactor rate limiting for REST/WebSocket (spot/futures).
- [ ] Crypto.com: Implement/refactor rate limiting for REST/WebSocket (spot/futures).

**Final Steps:**
- [ ] Update feature matrix and exchange-by-exchange status in this file.
- [ ] Ensure all rate limiting and backoff tests pass across all exchanges.
- [ ] Document exchange-specific rate limits, quotas, and reset periods.

---

## 🚧 TODO: Comprehensive Backtesting Framework

> **Goal:** Implement a high-performance, data-accurate backtesting framework for testing trading strategies against historical order book and trade data. Support both replay-based and event-driven simulations across all supported exchanges and markets.

**General Steps:**
- [x] Design a unified backtesting abstraction with clear interfaces for data sources, strategy inputs, and simulation outputs.
- [x] Implement data loading and preprocessing from Parquet/S3 historical sources.
- [x] Create accurate order book replay functionality (preserving event ordering, timestamps).
- [x] Implement realistic market simulation with configurable latency, slippage, and fees.
- [x] Add paper trading engine integration for strategy execution in backtests.
- [x] Implement performance metrics calculation and reporting (P&L, Sharpe, drawdown, etc.).
- [x] Add visualization and charting capabilities for backtest results.
- [x] Support parallel backtesting for parameter optimization and Monte Carlo simulations.
- [ ] Add/extend integration and unit tests for backtesting framework components.
- [x] Add/extend module-level and user-facing documentation.
- [x] Update `docs/IMPLEMENTATION_STATUS.md` with status and links.

**Feature-Specific TODOs:**

- [x] Historical Data Loading Framework (Parquet, S3, multi-exchange, spot/futures)
- [x] Order Book Replay Engine (timestamp-preserving, accurate sequencing)
- [x] Market Simulation (realistic order execution, fees, slippage)
- [x] Strategy Interface (event-driven, configurable parameters)
- [x] Performance Metrics (P&L, risk measures, trade statistics)
- [x] Visualization and Reporting (charts, tables, exports)
- [x] Parameter Optimization (grid search, genetic algorithms)
- [x] Multi-Exchange Simulation (cross-exchange strategies, arbitrage)

**Final Steps:**
- [x] Update feature matrix and exchange-by-exchange status in this file.
- [ ] Ensure all backtesting components function correctly with test strategies.
- [x] Document any limitations or assumptions in the simulation model.

---

## 🚧 TODO: Multi-Exchange Aggregation and Arbitrage Framework

> **Goal:** Implement a high-performance framework for real-time order book aggregation across multiple exchanges, enabling identification and execution of latency-sensitive arbitrage opportunities. Support both spot and futures markets with robust latency management and risk controls.

**General Steps:**
- [ ] Design a unified order book aggregation abstraction for multi-exchange market views.
- [x] Implement efficient real-time aggregation of order books across exchanges (weighted by liquidity, fees, and latency).
- [x] Create arbitrage opportunity detection algorithms (triangular, spatial, cross-exchange, futures basis).
- [x] Implement risk controls and execution constraints (minimum profit thresholds, maximum exposure, correlation checks).
- [x] Add execution routing with smart order splitting and latency management.
- [ ] Implement position tracking and risk monitoring across exchanges.
- [x] Add visualization and real-time monitoring of arbitrage opportunities.
- [ ] Support configurable execution strategies for different arbitrage types.
- [ ] Add/extend integration and unit tests for all arbitrage components.
- [ ] Add/extend module-level and user-facing documentation.
- [ ] Update `docs/IMPLEMENTATION_STATUS.md` with status and links.

**Feature-Specific TODOs:**

- [x] Multi-Exchange Order Book Aggregation (spot/futures, all supported exchanges)
- [x] Arbitrage Opportunity Detection (cross-exchange, triangular, futures basis)
- [x] Risk Management Framework (exposure limits, correlation checks, worst-case analysis)
- [x] Smart Execution Routing (latency-aware, fee-optimized)
- [x] Real-time Monitoring and Visualization
- [ ] Configurable Arbitrage Strategies (parameters, thresholds, execution tactics)
- [ ] Performance Metrics and Reporting (realized opportunities, missed opportunities, execution quality)

**Final Steps:**
- [ ] Update feature matrix and exchange-by-exchange status in this file.
- [ ] Ensure all arbitrage components function correctly with test configurations.
- [ ] Document any limitations, risks, or exchange-specific considerations.

---

## 🚧 TODO: Strategy Development Framework and Backtesting

> **Goal:** Implement a comprehensive framework for developing, testing, and deploying trading strategies. Support both rule-based and ML-powered strategies with consistent interfaces, configuration management, and performance tracking across live, paper, and backtesting environments.

**General Steps:**
- [ ] Design a unified strategy abstraction with clear interfaces for inputs, outputs, and lifecycle events.
- [ ] Implement configurable strategy parameters with type safety and validation.
- [ ] Create a strategy registry and discovery mechanism.
- [ ] Implement standard indicators and technical analysis tools.
- [ ] Add support for rule-based, event-driven strategy definitions.
- [ ] Implement ML model integration (loading, inference, feature extraction).
- [ ] Create a backtest runner with performance metrics and visualization.
- [ ] Add live/paper deployment capabilities with monitoring and control.
- [ ] Implement A/B testing and strategy comparison tools.
- [ ] Add/extend integration and unit tests for all strategy components.
- [ ] Add/extend module-level and user-facing documentation.
- [ ] Update `docs/IMPLEMENTATION_STATUS.md` with status and links.

**Feature-Specific TODOs:**

- [x] Strategy Interface Definition (inputs, outputs, events, lifecycle)
 - [x] Technical Analysis Library (indicators, patterns, signals)
- [ ] ML Integration Framework (feature extraction, model loading, inference)
- [ ] Strategy Configuration and Parameter Management
- [ ] Backtest Runner and Performance Evaluation
- [ ] Live/Paper Deployment and Monitoring
- [ ] A/B Testing and Strategy Comparison
- [ ] Documentation and Example Strategies

**Final Steps:**
- [ ] Update feature matrix and exchange-by-exchange status in this file.
- [ ] Ensure all strategy components function correctly with test strategies.
- [ ] Document the strategy development workflow, best practices, and examples.

---

## 🚧 TODO: Advanced Risk Management Framework

> **Goal:** Implement a comprehensive risk management framework for trading activities across all supported exchanges and markets. Support position limits, drawdown controls, correlation-based exposure management, and automated risk mitigation actions with robust monitoring and alerting.

**General Steps:**
 - [x] Design a unified risk management abstraction with configurable rules and actions.
 - [x] Implement position and exposure tracking across exchanges and instruments.
 - [x] Create drawdown and loss limit controls with configurable thresholds.
 - [x] Implement correlation-based exposure management for related instruments.
 - [ ] Add volatility-adjusted position sizing and risk scaling.
 - [ ] Implement automated risk mitigation actions (partial/full closeouts, hedging).
 - [ ] Create real-time risk dashboards and monitoring.
 - [x] Add alerting and notification for risk threshold violations.
 - [ ] Implement stress testing and scenario analysis tools.
 - [x] Add/extend integration and unit tests for all risk management components.
 - [x] Add/extend module-level and user-facing documentation.
 - [x] Update `docs/IMPLEMENTATION_STATUS.md` with status and links.

**Feature-Specific TODOs:**

 - [x] Position and Exposure Tracking (multi-exchange, spot/futures)
 - [x] Drawdown and Loss Limit Controls
 - [x] Correlation-Based Exposure Management
 - [x] Automated Risk Mitigation Actions
 - [x] Volatility-Adjusted Position Sizing

- [ ] Real-time Risk Monitoring and Dashboards
 - [x] Alerting and Notification System
- [ ] Stress Testing and Scenario Analysis

**Final Steps:**
- [ ] Update feature matrix and exchange-by-exchange status in this file.
- [ ] Ensure all risk management components function correctly with test configurations.
- [ ] Document the risk framework, configuration options, and best practices.

---

## 🚧 TODO: Market Making Engine

> **Goal:** Implement a high-performance market making engine for providing liquidity across all supported exchanges and markets. Support advanced features like inventory management, skew adjustment, spread optimization, and adverse selection mitigation with robust risk controls and performance tracking.

**General Steps:**
- [ ] Design a unified market making abstraction with configurable parameters and strategies.
- [x] Implement efficient two-sided quote management (bid/ask placement, monitoring, adjustment).
- [x] Create inventory management and skew adjustment algorithms.
- [x] Implement spread optimization based on volatility, competition, and flow toxicity.
- [x] Add adverse selection detection and mitigation tactics.
- [x] Implement quote refresh and positioning strategies (layering, reactive, predictive).
- [x] Create performance tracking and PnL attribution (spread capture, inventory, funding).
- [x] Add risk controls and circuit breakers for market conditions and inventory extremes.
- [x] Implement visualization and monitoring of market making activities.
- [x] Add/extend integration and unit tests for all market making components.
- [x] Add/extend module-level and user-facing documentation.
- [x] Update `docs/IMPLEMENTATION_STATUS.md` with status and links.

**Feature-Specific TODOs:**

- [x] Two-Sided Quote Management (all exchanges, spot/futures)
- [x] Inventory Management and Skew Adjustment
- [x] Spread Optimization Algorithms
- [x] Adverse Selection Detection and Mitigation
- [x] Quote Refresh and Positioning Strategies
- [x] Performance Tracking and PnL Attribution
- [x] Risk Controls and Circuit Breakers
- [x] Visualization and Monitoring Tools

**Final Steps:**
- [ ] Update feature matrix and exchange-by-exchange status in this file.
- [ ] Ensure all market making components function correctly with test configurations.
- [ ] Document the market making framework, parameters, and strategy examples.

#
---

**This file is the single source of truth for the implementation status of all features and components in the jackbot project. All contributors must update this file when making changes to the codebase.**

---
