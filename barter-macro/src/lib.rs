//! # barter-macro
//!
//! Procedural macros for the Barter ecosystem, providing compile-time code generation
//! to reduce boilerplate and eliminate manual synchronization across multiple code locations.
//!
//! ## Available Macros
//!
//! ### Derive Macros
//!
//! - [`DeExchange`] - Derives `serde::Deserialize` for exchange marker types
//! - [`SerExchange`] - Derives `serde::Serialize` for exchange marker types
//! - [`DeSubKind`] - Derives `serde::Deserialize` for subscription kind types
//! - [`SerSubKind`] - Derives `serde::Serialize` for subscription kind types
//!
//! ### Procedural Macros
//!
//! - [`define_stream_connectors!`] - Generates stream connector dispatch code for `DynamicStreams`,
//!   consolidating exchange/subscription kind registration into a single declarative table.
//!
//! ## Example
//!
//! ```rust,ignore
//! use barter_macro::{DeExchange, SerExchange, define_stream_connectors};
//!
//! // Derive macros for exchange types
//! #[derive(Default, DeExchange, SerExchange)]
//! pub struct BinanceSpot;
//!
//! // Stream connector registration
//! define_stream_connectors! {
//!     BinanceSpot => [PublicTrades, OrderBooksL1, OrderBooksL2],
//!     Coinbase => [PublicTrades],
//! }
//! ```

extern crate proc_macro;

use convert_case::{Boundary, Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use syn::DeriveInput;

#[proc_macro_derive(DeExchange)]
pub fn de_exchange_derive(input: TokenStream) -> TokenStream {
    // Parse Rust code abstract syntax tree with Syn from TokenStream -> DeriveInput
    let ast: DeriveInput =
        syn::parse(input).expect("de_exchange_derive() failed to parse input TokenStream");

    // Determine execution name
    let exchange = &ast.ident;

    let generated = quote! {
        impl<'de> serde::Deserialize<'de> for #exchange {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::de::Deserializer<'de>
            {
                let input = <String as serde::Deserialize>::deserialize(deserializer)?;
                let exchange = #exchange::ID;
                let expected = exchange.as_str();

                if input.as_str() == expected {
                    Ok(Self::default())
                } else {
                    Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(input.as_str()),
                        &expected
                    ))
                }
            }
        }
    };

    TokenStream::from(generated)
}

#[proc_macro_derive(SerExchange)]
pub fn ser_exchange_derive(input: TokenStream) -> TokenStream {
    // Parse Rust code abstract syntax tree with Syn from TokenStream -> DeriveInput
    let ast: DeriveInput =
        syn::parse(input).expect("ser_exchange_derive() failed to parse input TokenStream");

    // Determine Exchange
    let exchange = &ast.ident;

    let generated = quote! {
        impl serde::Serialize for #exchange {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::ser::Serializer,
            {
                serializer.serialize_str(#exchange::ID.as_str())
            }
        }
    };

    TokenStream::from(generated)
}

#[proc_macro_derive(DeSubKind)]
pub fn de_sub_kind_derive(input: TokenStream) -> TokenStream {
    // Parse Rust code abstract syntax tree with Syn from TokenStream -> DeriveInput
    let ast: DeriveInput =
        syn::parse(input).expect("de_sub_kind_derive() failed to parse input TokenStream");

    // Determine SubKind name
    let sub_kind = &ast.ident;

    let expected_sub_kind = sub_kind
        .to_string()
        .from_case(Case::Pascal)
        .without_boundaries(&Boundary::letter_digit())
        .to_case(Case::Snake);

    let generated = quote! {
        impl<'de> serde::Deserialize<'de> for #sub_kind {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::de::Deserializer<'de>
            {
                let input = <String as serde::Deserialize>::deserialize(deserializer)?;

                if input == #expected_sub_kind {
                    Ok(Self)
                } else {
                    Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(input.as_str()),
                        &#expected_sub_kind
                    ))
                }
            }
        }
    };

    TokenStream::from(generated)
}

#[proc_macro_derive(SerSubKind)]
pub fn ser_sub_kind_derive(input: TokenStream) -> TokenStream {
    // Parse Rust code abstract syntax tree with Syn from TokenStream -> DeriveInput
    let ast: DeriveInput =
        syn::parse(input).expect("ser_sub_kind_derive() failed to parse input TokenStream");

    // Determine SubKind name
    let sub_kind = &ast.ident;
    let sub_kind_string = sub_kind.to_string().to_case(Case::Snake);
    let sub_kind_str = sub_kind_string.as_str();

    let generated = quote! {
        impl serde::Serialize for #sub_kind {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::ser::Serializer,
            {
                serializer.serialize_str(#sub_kind_str)
            }
        }
    };

    TokenStream::from(generated)
}

mod stream_registry;
use stream_registry::StreamConnectorsInput;

/// Generates stream connector dispatch code for `DynamicStreams`.
///
/// This macro consolidates stream connector registration into a single declarative table,
/// auto-generating match arms, where clauses, and imports to eliminate manual synchronization
/// across multiple locations when adding or modifying exchange connectors.
///
/// ## Generated Code
///
/// The macro generates:
/// - **Import statements** for all connector, market, and channel types
/// - **Match arms** dispatching `(ExchangeId, SubKind)` to connector initialization
/// - **Where clause bounds** for the `init` function
/// - **Complete `impl` block** for `DynamicStreams::init`
///
/// ## Input Syntax
///
/// ```rust,ignore
/// define_stream_connectors! {
///     ConnectorType => [SubscriptionKind1, SubscriptionKind2, ...],
///     // ... more connectors
/// }
/// ```
///
/// ## Example
///
/// ```rust,ignore
/// define_stream_connectors! {
///     BinanceSpot => [PublicTrades, OrderBooksL1, OrderBooksL2],
///     BinanceFuturesUsd => [PublicTrades, OrderBooksL1, OrderBooksL2, Liquidations],
///     BybitSpot => [PublicTrades, OrderBooksL1, OrderBooksL2],
///     BybitPerpetualsUsd => [PublicTrades, OrderBooksL1, OrderBooksL2],
///     Bitfinex => [PublicTrades],
///     Bitmex => [PublicTrades],
///     Coinbase => [PublicTrades],
///     GateioSpot => [PublicTrades],
///     GateioFuturesUsd => [PublicTrades],
///     GateioFuturesBtc => [PublicTrades],
///     GateioPerpetualsBtc => [PublicTrades],
///     GateioPerpetualsUsd => [PublicTrades],
///     GateioOptions => [PublicTrades],
///     Kraken => [PublicTrades, OrderBooksL1],
///     Okx => [PublicTrades],
///     Poloniex => [PublicTrades],
/// }
/// ```
///
/// ## Naming Conventions
///
/// The macro derives associated types from connector names using these conventions:
///
/// | Connector | ExchangeId | Market Type | Module Path |
/// |-----------|------------|-------------|-------------|
/// | `BinanceSpot` | `ExchangeId::BinanceSpot` | `BinanceMarket` | `binance::spot` |
/// | `BinanceFuturesUsd` | `ExchangeId::BinanceFuturesUsd` | `BinanceMarket` | `binance::futures` |
/// | `BybitSpot` | `ExchangeId::BybitSpot` | `BybitMarket` | `bybit::spot` |
/// | `BybitPerpetualsUsd` | `ExchangeId::BybitPerpetualsUsd` | `BybitMarket` | `bybit::futures` |
/// | `Bitfinex` | `ExchangeId::Bitfinex` | `BitfinexMarket` | `bitfinex` |
/// | `GateioSpot` | `ExchangeId::GateioSpot` | `GateioMarket` | `gateio::spot` |
/// | ... | ... | ... | ... |
///
/// ## Subscription Kinds
///
/// Supported subscription kinds and their channel field mappings:
///
/// | Kind | Channel Field |
/// |------|---------------|
/// | `PublicTrades` | `trades` |
/// | `OrderBooksL1` | `l1s` |
/// | `OrderBooksL2` | `l2s` |
/// | `Liquidations` | `liquidations` |
///
/// ## Adding a New Exchange
///
/// To add support for a new exchange:
///
/// 1. Implement the exchange connector module in `barter-data/src/exchange/`
/// 2. Add a single line to the `define_stream_connectors!` invocation:
///    ```rust,ignore
///    NewExchange => [PublicTrades, OrderBooksL1],
///    ```
/// 3. The macro will automatically generate all necessary imports, match arms,
///    and where clause bounds.
///
/// ## Compile-Time Validation
///
/// The macro performs compile-time validation and emits clear error messages for:
///
/// - **Duplicate registrations**: Same `(Connector, Kind)` pair appears twice
/// - **Unknown subscription kinds**: Kind is not one of the supported types
/// - **Unknown connector types**: Connector name doesn't match known patterns
/// - **Empty kinds list**: A connector must support at least one subscription kind
///
/// ## Error Examples
///
/// ```text
/// error: duplicate registration for (BinanceSpot, PublicTrades)
///   --> src/streams/builder/dynamic/mod.rs:25:5
///    |
/// 25 |     BinanceSpot => [PublicTrades],
///    |                     ^^^^^^^^^^^^
/// ```
///
/// ```text
/// error: Unknown subscription kind: InvalidKind. Expected one of: ["PublicTrades", "OrderBooksL1", "OrderBooksL2", "Liquidations"]
///   --> src/streams/builder/dynamic/mod.rs:26:20
///    |
/// 26 |     BinanceSpot => [InvalidKind],
///    |                     ^^^^^^^^^^^
/// ```
#[proc_macro]
pub fn define_stream_connectors(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as StreamConnectorsInput);

    if let Err(err) = input.validate() {
        return err.to_compile_error().into();
    }

    match input.generate() {
        Ok(expanded) => expanded.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

