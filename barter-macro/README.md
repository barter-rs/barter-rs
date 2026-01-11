# barter-macro

Procedural macros for the [Barter](https://github.com/barter-rs/barter-rs) trading ecosystem, providing compile-time code generation to reduce boilerplate and eliminate manual synchronization across multiple code locations.

## Overview

This crate provides derive macros and procedural macros used throughout the Barter ecosystem:

- **Derive macros** for serialization/deserialization of exchange and subscription kind types
- **Procedural macros** for dynamic stream connector registration and dispatch

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
barter-macro = { version = "0.1", path = "../barter-macro" }
```

## Available Macros

### Derive Macros

#### `DeExchange` / `SerExchange`

Derives `serde::Deserialize` and `serde::Serialize` implementations for exchange marker types. These expect the type to have an associated `ID` constant of type `ExchangeId`.

```rust
use barter_macro::{DeExchange, SerExchange};

#[derive(Default, DeExchange, SerExchange)]
pub struct BinanceSpot;

impl BinanceSpot {
    pub const ID: ExchangeId = ExchangeId::BinanceSpot;
}
```

#### `DeSubKind` / `SerSubKind`

Derives `serde::Deserialize` and `serde::Serialize` implementations for subscription kind types. Converts between PascalCase type names and snake_case string representations.

```rust
use barter_macro::{DeSubKind, SerSubKind};

#[derive(DeSubKind, SerSubKind)]
pub struct PublicTrades;
// Serializes to: "public_trades"
// Deserializes from: "public_trades"

#[derive(DeSubKind, SerSubKind)]
pub struct OrderBooksL1;
// Serializes to: "order_books_l1"
// Deserializes from: "order_books_l1"
```

### Procedural Macros

#### `define_stream_connectors!`

Generates stream connector dispatch code for `DynamicStreams`, consolidating exchange/subscription kind registration into a single declarative table.

**Problem it solves:** Previously, adding a new exchange connector required synchronizing changes across three locations:
1. Import statements (~15 lines)
2. Match arms (~25 arms)
3. Where clause constraints (~25 bounds)

With `define_stream_connectors!`, you add a single line and the macro generates everything else.

**Usage:**

```rust
use barter_macro::define_stream_connectors;

define_stream_connectors! {
    // Connector => [SupportedKinds...]
    BinanceSpot => [PublicTrades, OrderBooksL1, OrderBooksL2],
    BinanceFuturesUsd => [PublicTrades, OrderBooksL1, OrderBooksL2, Liquidations],
    BybitSpot => [PublicTrades, OrderBooksL1, OrderBooksL2],
    BybitPerpetualsUsd => [PublicTrades, OrderBooksL1, OrderBooksL2],
    Bitfinex => [PublicTrades],
    Bitmex => [PublicTrades],
    Coinbase => [PublicTrades],
    GateioSpot => [PublicTrades],
    GateioFuturesUsd => [PublicTrades],
    GateioFuturesBtc => [PublicTrades],
    GateioPerpetualsBtc => [PublicTrades],
    GateioPerpetualsUsd => [PublicTrades],
    GateioOptions => [PublicTrades],
    Kraken => [PublicTrades, OrderBooksL1],
    Okx => [PublicTrades],
    Poloniex => [PublicTrades],
}
```

**Generated code includes:**
- Import statements for all connector and market types
- `impl DynamicStreams::init` with:
  - All necessary where clause bounds
  - Match arms for each `(ExchangeId, SubKind)` combination
  - Fallback for unsupported combinations

## Adding a New Exchange Connector

To add support for a new exchange:

1. **Implement the exchange connector** in `barter-data/src/exchange/`:
   - Create the connector type (e.g., `NewExchangeSpot`)
   - Implement required traits (`Subscriber`, `Transformer`, etc.)
   - Add market and channel types

2. **Register the connector** by adding one line to the `define_stream_connectors!` invocation:
   ```rust
   NewExchangeSpot => [PublicTrades, OrderBooksL1],
   ```

3. **Update `ConnectorMetadata::from_ident`** in `barter-macro/src/stream_registry.rs` if the connector doesn't follow standard naming conventions:
   ```rust
   "NewExchangeSpot" => ("new_exchange", Some("spot"), "NewExchangeMarket"),
   ```

That's it! The macro automatically generates all imports, match arms, and where clause bounds.

## Subscription Kinds

The following subscription kinds are supported:

| Kind           | Channel Field  | Description                       |
| -------------- | -------------- | --------------------------------- |
| `PublicTrades` | `trades`       | Real-time trade data              |
| `OrderBooksL1` | `l1s`          | Level 1 order book (best bid/ask) |
| `OrderBooksL2` | `l2s`          | Level 2 order book (depth)        |
| `Liquidations` | `liquidations` | Liquidation events                |

## Naming Conventions

The macro derives associated types from connector names:

| Connector Pattern     | Exchange Root | Sub-Module  | Market Type      |
| --------------------- | ------------- | ----------- | ---------------- |
| `BinanceSpot`         | `binance`     | `spot`      | `BinanceMarket`  |
| `BinanceFuturesUsd`   | `binance`     | `futures`   | `BinanceMarket`  |
| `BybitSpot`           | `bybit`       | `spot`      | `BybitMarket`    |
| `BybitPerpetualsUsd`  | `bybit`       | `futures`   | `BybitMarket`    |
| `Bitfinex`            | `bitfinex`    | (none)      | `BitfinexMarket` |
| `GateioSpot`          | `gateio`      | `spot`      | `GateioMarket`   |
| `GateioFuturesUsd`    | `gateio`      | `future`    | `GateioMarket`   |
| `GateioPerpetualsBtc` | `gateio`      | `perpetual` | `GateioMarket`   |
| `GateioOptions`       | `gateio`      | `option`    | `GateioMarket`   |

## Compile-Time Validation

The macro validates input at compile time and provides clear error messages:

### Duplicate Registration
```text
error: Duplicate registration for (BinanceSpot, PublicTrades)
  --> src/streams/builder/dynamic/mod.rs:25:20
   |
25 |     BinanceSpot => [PublicTrades],
   |                     ^^^^^^^^^^^^
```

### Unknown Subscription Kind
```text
error: Unknown subscription kind: InvalidKind. Expected one of: ["PublicTrades", "OrderBooksL1", "OrderBooksL2", "Liquidations"]
  --> src/streams/builder/dynamic/mod.rs:26:20
   |
26 |     BinanceSpot => [InvalidKind],
   |                     ^^^^^^^^^^^
```

### Unknown Connector Type
```text
error: Unknown connector type: InvalidConnector
  --> src/streams/builder/dynamic/mod.rs:27:5
   |
27 |     InvalidConnector => [PublicTrades],
   |     ^^^^^^^^^^^^^^^^
```

### Empty Kinds List
```text
error: Connector must support at least one subscription kind
  --> src/streams/builder/dynamic/mod.rs:28:5
   |
28 |     BinanceSpot => [],
   |     ^^^^^^^^^^^
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
