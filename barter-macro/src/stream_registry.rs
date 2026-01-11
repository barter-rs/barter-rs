//! Stream connector registry implementation for the `define_stream_connectors!` macro.
//!
//! This module provides the parsing, validation, and code generation logic for the
//! `define_stream_connectors!` procedural macro. It transforms a declarative table of
//! exchange connectors and their supported subscription kinds into complete Rust code
//! for dynamic stream dispatch.
//!
//! ## Architecture
//!
//! The implementation follows these stages:
//!
//! 1. **Parsing** ([`StreamConnectorsInput`]): Parses the macro input syntax into structured data
//! 2. **Validation** ([`StreamConnectorsInput::validate`]): Validates for duplicates and unknown types
//! 3. **Metadata Resolution** ([`ConnectorMetadata`]): Derives module paths, market types, and channel types
//! 4. **Code Generation** ([`StreamConnectorsInput::generate`]): Produces the final token stream
//!
//! ## Supported Connectors
//!
//! The following connector types are recognized and mapped to their exchange families:
//!
//! | Exchange Family | Connectors | Market Type |
//! |-----------------|------------|-------------|
//! | Binance | `BinanceSpot`, `BinanceFuturesUsd` | `BinanceMarket` |
//! | Bybit | `BybitSpot`, `BybitPerpetualsUsd` | `BybitMarket` |
//! | Bitfinex | `Bitfinex` | `BitfinexMarket` |
//! | Bitmex | `Bitmex` | `BitmexMarket` |
//! | Coinbase | `Coinbase` | `CoinbaseMarket` |
//! | Gate.io | `GateioSpot`, `GateioFuturesUsd`, `GateioFuturesBtc`, `GateioPerpetualsBtc`, `GateioPerpetualsUsd`, `GateioOptions` | `GateioMarket` |
//! | Kraken | `Kraken` | `KrakenMarket` |
//! | OKX | `Okx` | `OkxMarket` |
//! | Poloniex | `Poloniex` | `PoloniexMarket` |
//!
//! ## Adding New Connectors
//!
//! To add support for a new connector:
//!
//! 1. Add a match arm in [`ConnectorMetadata::from_ident`] with the connector name,
//!    exchange root, sub-module (if any), and market type
//! 2. The macro invocation site will then accept the new connector

use std::collections::{HashSet, BTreeMap};
use syn::parse::{Parse, ParseStream};
use syn::{bracketed, Ident, Result, Token, Error};
use syn::punctuated::Punctuated;
use quote::{quote, format_ident};
use proc_macro2::TokenStream;
use convert_case::{Case, Casing};

/// A single connector registration entry parsed from the macro input.
///
/// Represents a line like `BinanceSpot => [PublicTrades, OrderBooksL1]` in the
/// `define_stream_connectors!` macro invocation.
///
/// # Example
///
/// ```text
/// BinanceSpot => [PublicTrades, OrderBooksL1, OrderBooksL2]
/// ```
///
/// Parses into:
/// - `connector`: `BinanceSpot`
/// - `kinds`: `[PublicTrades, OrderBooksL1, OrderBooksL2]`
pub struct ConnectorEntry {
    /// The connector type name (e.g., `BinanceSpot`)
    pub connector: Ident,
    /// List of supported subscription kinds
    pub kinds: Vec<Ident>,
}

impl Parse for ConnectorEntry {
    fn parse(input: ParseStream) -> Result<Self> {
        let connector: Ident = input.parse()?;
        input.parse::<Token![=>]>()?;
        
        let content;
        bracketed!(content in input);
        
        let kinds_punctuated: Punctuated<Ident, Token![,]> = content.parse_terminated(Parse::parse, Token![,])?;
        let kinds = kinds_punctuated.into_iter().collect();
        
        Ok(ConnectorEntry {
            connector,
            kinds,
        })
    }
}

/// The complete parsed macro input containing all connector registrations.
///
/// This struct is the entry point for the macro's parsing logic. It implements
/// [`syn::parse::Parse`] to parse the entire `define_stream_connectors!` invocation.
///
/// # Example Input
///
/// ```rust,ignore
/// BinanceSpot => [PublicTrades, OrderBooksL1],
/// Coinbase => [PublicTrades],
/// ```
///
/// # Validation
///
/// After parsing, call [`validate()`](Self::validate) to check for:
/// - Duplicate `(connector, kind)` combinations
/// - Unknown subscription kinds
/// - Empty kinds lists
///
/// # Code Generation
///
/// After validation, call [`generate()`](Self::generate) to produce the output token stream.
pub struct StreamConnectorsInput {
    pub entries: Vec<ConnectorEntry>,
}

impl Parse for StreamConnectorsInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let entries_punctuated: Punctuated<ConnectorEntry, Token![,]> = input.parse_terminated(ConnectorEntry::parse, Token![,])?;
        let entries = entries_punctuated.into_iter().collect();
        Ok(StreamConnectorsInput { entries })
    }
}

impl StreamConnectorsInput {
    /// Validates the parsed input for correctness.
    ///
    /// # Validations Performed
    ///
    /// 1. **Duplicate detection**: Ensures no `(connector, kind)` pair appears twice
    /// 2. **Known kinds**: Validates that all subscription kinds are recognized
    /// 3. **Non-empty kinds**: Each connector must support at least one subscription kind
    ///
    /// # Errors
    ///
    /// Returns a combined [`syn::Error`] containing all validation failures with
    /// appropriate span information for IDE integration.
    ///
    /// # Example Error Messages
    ///
    /// ```text
    /// error: Duplicate registration for (BinanceSpot, PublicTrades)
    /// error: Unknown subscription kind: InvalidKind. Expected one of: ["PublicTrades", "OrderBooksL1", "OrderBooksL2", "Liquidations"]
    /// error: Connector must support at least one subscription kind
    /// ```
    pub fn validate(&self) -> Result<()> {
        let mut errors = Vec::new();
        let mut seen_combinations = HashSet::new();

        let valid_kinds = ["PublicTrades", "OrderBooksL1", "OrderBooksL2", "Liquidations"];

        for entry in &self.entries {
            if entry.kinds.is_empty() {
                errors.push(Error::new(entry.connector.span(), "Connector must support at least one subscription kind"));
            }

            for kind in &entry.kinds {
                let kind_str = kind.to_string();
                
                // Validate known kinds
                if !valid_kinds.contains(&kind_str.as_str()) {
                    errors.push(Error::new(kind.span(), format!("Unknown subscription kind: {}. Expected one of: {:?}", kind_str, valid_kinds)));
                }

                // Validate duplicate combinations
                let combination = (entry.connector.to_string(), kind_str);
                if !seen_combinations.insert(combination.clone()) {
                     errors.push(Error::new(kind.span(), format!("Duplicate registration for ({}, {})", combination.0, combination.1)));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
             let mut combined_error = errors[0].clone();
             for err in errors.into_iter().skip(1) {
                 combined_error.combine(err);
             }
             Err(combined_error)
        }
    }
}

/// Metadata derived from a connector type name.
///
/// This struct holds all the associated information needed to generate code for a connector,
/// including module paths, market types, and channel types. The information is derived from
/// the connector name using naming conventions.
///
/// # Fields
///
/// - `exchange_root`: The base exchange module name (e.g., `"binance"`, `"gateio"`)
/// - `sub_module`: Optional sub-module for exchange variants (e.g., `Some("spot")`, `Some("futures")`)
/// - `market`: The market identifier type (e.g., `BinanceMarket`)
/// - `channel`: The channel identifier type (e.g., `BinanceChannel`)
struct ConnectorMetadata {
    exchange_root: String,
    sub_module: Option<String>,
    market: Ident,
    channel: Ident,
}

impl ConnectorMetadata {
    /// Derives connector metadata from a connector type identifier.
    ///
    /// # Arguments
    ///
    /// * `ident` - The connector type name (e.g., `BinanceSpot`, `GateioFuturesUsd`)
    ///
    /// # Returns
    ///
    /// Returns `Ok(ConnectorMetadata)` with the derived information, or an error
    /// if the connector type is not recognized.
    ///
    /// # Supported Connectors
    ///
    /// See module-level documentation for the complete list of supported connectors.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let meta = ConnectorMetadata::from_ident(&quote::format_ident!("BinanceSpot"))?;
    /// assert_eq!(meta.exchange_root, "binance");
    /// assert_eq!(meta.sub_module, Some("spot".to_string()));
    /// assert_eq!(meta.market.to_string(), "BinanceMarket");
    /// ```
    fn from_ident(ident: &Ident) -> Result<Self> {
        let s = ident.to_string();
        let span = ident.span();

        let (exchange_root, sub_module, market_name) = match s.as_str() {
            "BinanceSpot" => ("binance", Some("spot"), "BinanceMarket"),
            "BinanceFuturesUsd" => ("binance", Some("futures"), "BinanceMarket"),
            "BybitSpot" => ("bybit", Some("spot"), "BybitMarket"),
            "BybitPerpetualsUsd" => ("bybit", Some("futures"), "BybitMarket"),
            "Bitfinex" => ("bitfinex", None, "BitfinexMarket"),
            "Bitmex" => ("bitmex", None, "BitmexMarket"),
            "Coinbase" => ("coinbase", None, "CoinbaseMarket"),
            "GateioSpot" => ("gateio", Some("spot"), "GateioMarket"),
            "GateioFuturesUsd" => ("gateio", Some("future"), "GateioMarket"),
            "GateioFuturesBtc" => ("gateio", Some("future"), "GateioMarket"),
            "GateioPerpetualsBtc" => ("gateio", Some("perpetual"), "GateioMarket"),
            "GateioPerpetualsUsd" => ("gateio", Some("perpetual"), "GateioMarket"),
            "GateioOptions" => ("gateio", Some("option"), "GateioMarket"),
            "Kraken" => ("kraken", None, "KrakenMarket"),
            "Okx" => ("okx", None, "OkxMarket"),
            "Poloniex" => ("poloniex", None, "PoloniexMarket"),
            _ => return Err(Error::new(span, format!("Unknown connector type: {}", s))),
        };

        let channel_name = format!("{}Channel", exchange_root.to_case(Case::Pascal));

        Ok(Self {
            exchange_root: exchange_root.to_string(),
            sub_module: sub_module.map(|s| s.to_string()),
            market: Ident::new(market_name, span),
            channel: Ident::new(&channel_name, span),
        })
    }
}

impl StreamConnectorsInput {
    /// Generates the complete output token stream for the macro.
    ///
    /// This method transforms the parsed and validated input into Rust code that:
    ///
    /// 1. **Imports** all necessary connector, market, and channel types
    /// 2. **Implements** `DynamicStreams::init` with:
    ///    - Where clause bounds for all connector/kind combinations
    ///    - Match arms dispatching to the correct connector initialization
    ///    - Channel selection based on subscription kind
    ///
    /// # Generated Code Structure
    ///
    /// ```rust,ignore
    /// use crate::exchange::{
    ///     binance::{spot::BinanceSpot, futures::BinanceFuturesUsd, market::BinanceMarket, channel::BinanceChannel},
    ///     // ... other exchanges
    /// };
    ///
    /// impl<InstrumentKey> DynamicStreams<InstrumentKey> {
    ///     pub async fn init<SubBatchIter, SubIter, Sub, Instrument>(
    ///         subscription_batches: SubBatchIter,
    ///     ) -> Result<Self, DataError>
    ///     where
    ///         // Base constraints...
    ///         Subscription<BinanceSpot, Instrument, PublicTrades>: Identifier<BinanceMarket> + Identifier<BinanceChannel>,
    ///         // ... more bounds
    ///     {
    ///         // Implementation with match dispatch
    ///     }
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if any connector metadata cannot be resolved (unknown connector type).
    pub fn generate(&self) -> Result<TokenStream> {
        let mut imports_map: BTreeMap<String, (String, String, BTreeMap<Option<String>, Vec<Ident>>)> = BTreeMap::new();
        let mut match_arms = TokenStream::new();
        let mut where_bounds = TokenStream::new();

        for entry in &self.entries {
            let meta = ConnectorMetadata::from_ident(&entry.connector)?;

            // Collect imports
            let exchange_entry = imports_map.entry(meta.exchange_root.clone())
                .or_insert_with(|| (meta.market.to_string(), meta.channel.to_string(), BTreeMap::new()));
            
            exchange_entry.2.entry(meta.sub_module.clone())
                .or_insert_with(Vec::new)
                .push(entry.connector.clone());


            let exchange_id = format_ident!("{}", entry.connector);
            let market = &meta.market;
            let channel = &meta.channel;
            let connector = &entry.connector;

            for kind in &entry.kinds {
                let channel_field = match kind.to_string().as_str() {
                    "PublicTrades" => format_ident!("trades"),
                    "OrderBooksL1" => format_ident!("l1s"),
                    "OrderBooksL2" => format_ident!("l2s"),
                    "Liquidations" => format_ident!("liquidations"),
                    _ => return Err(Error::new(kind.span(), "Unknown kind")),
                };

                // Match Arm
                let match_arm = quote! {
                    (ExchangeId::#exchange_id, SubKind::#kind) => {
                        init_and_forward::<_, _, #kind>(
                            #connector::default(),
                            subs,
                            txs.#channel_field.get(&ExchangeId::#exchange_id).expect("channel must exist").clone(),
                            #kind
                        ).await
                    }
                };
                match_arms.extend(match_arm);

                // Where Bound
                let where_bound = quote! {
                    Subscription<#connector, Instrument, #kind>: Identifier<#market> + Identifier<#channel>,
                };
                where_bounds.extend(where_bound);
            }
        }

        // Generate Imports
        let mut imports = TokenStream::new();
        for (root, (market_name, channel_name, sub_modules)) in imports_map {
             let root_ident = format_ident!("{}", root);
             let market_ident = format_ident!("{}", market_name);
             let channel_ident = format_ident!("{}", channel_name);
             
             let mut sub_imports = TokenStream::new();
             sub_imports.extend(quote! { market::#market_ident, channel::#channel_ident, });

             for (sub_mod, connectors) in sub_modules {
                 if let Some(sub) = sub_mod {
                     let sub_ident = format_ident!("{}", sub);
                     let connectors_iter = connectors.iter();
                     sub_imports.extend(quote! { #sub_ident::{ #(#connectors_iter),* }, });
                 } else {
                     let connectors_iter = connectors.iter();
                     sub_imports.extend(quote! { #(#connectors_iter),*, });
                 }
             }

             imports.extend(quote! {
                 #root_ident::{ #sub_imports },
             });
        }
        
        Ok(quote! {
             use crate::exchange::{ #imports };

             impl<InstrumentKey> DynamicStreams<InstrumentKey> {
                 pub async fn init<SubBatchIter, SubIter, Sub, Instrument>(
                    subscription_batches: SubBatchIter,
                 ) -> Result<Self, DataError>
                 where
                    SubBatchIter: IntoIterator<Item = SubIter>,
                    SubIter: IntoIterator<Item = Sub>,
                    Sub: Into<Subscription<ExchangeId, Instrument, SubKind>>,
                    Instrument: InstrumentData<Key = InstrumentKey> + Ord + Display + 'static,
                    InstrumentKey: Debug + Clone + PartialEq + Send + Sync + 'static,
                    #where_bounds
                 {
                    let batches = validate_batches(subscription_batches)?;
                    let channels = Channels::try_from(&batches)?;

                    let futures =
                        batches.into_iter().map(|mut batch| {
                            batch.sort_unstable_by_key(|sub| (sub.exchange, sub.kind));
                            let by_exchange_by_sub_kind =
                                batch.into_iter().chunk_by(|sub| (sub.exchange, sub.kind));

                            let batch_futures =
                                by_exchange_by_sub_kind
                                    .into_iter()
                                    .map(|((exchange, sub_kind), subs)| {
                                        let subs = subs.into_iter().collect::<Vec<_>>();
                                        let txs = Arc::clone(&channels.txs);
                                        async move {
                                            match (exchange, sub_kind) {
                                                #match_arms
                                                (exchange, sub_kind) => Err(DataError::Unsupported { 
                                                    exchange, 
                                                    sub_kind 
                                                }),
                                            }
                                        }
                                    });

                            try_join_all(batch_futures)
                        });

                    try_join_all(futures).await?;

                    Ok(Self {
                        trades: channels
                            .rxs
                            .trades
                            .into_iter()
                            .map(|(exchange, rx)| (exchange, rx.into_stream()))
                            .collect(),
                        l1s: channels
                            .rxs
                            .l1s
                            .into_iter()
                            .map(|(exchange, rx)| (exchange, rx.into_stream()))
                            .collect(),
                        l2s: channels
                            .rxs
                            .l2s
                            .into_iter()
                            .map(|(exchange, rx)| (exchange, rx.into_stream()))
                            .collect(),
                        liquidations: channels
                            .rxs
                            .liquidations
                            .into_iter()
                            .map(|(exchange, rx)| (exchange, rx.into_stream()))
                            .collect(),
                    })
                 }
             }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_parse_simple() {
        let input: StreamConnectorsInput = parse_quote! {
            BinanceSpot => [PublicTrades, OrderBooksL1],
            Coinbase => [PublicTrades]
        };
        assert_eq!(input.entries.len(), 2);
        assert_eq!(input.entries[0].connector.to_string(), "BinanceSpot");
        assert_eq!(input.entries[0].kinds.len(), 2);
    }

    #[test]
    fn test_validate_valid() {
        let input: StreamConnectorsInput = parse_quote! {
            BinanceSpot => [PublicTrades]
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_validate_duplicate_kind() {
        let input: StreamConnectorsInput = parse_quote! {
            BinanceSpot => [PublicTrades, PublicTrades]
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_validate_duplicate_entry() {
        let input: StreamConnectorsInput = parse_quote! {
            BinanceSpot => [PublicTrades],
            BinanceSpot => [PublicTrades]
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_validate_unknown_kind() {
        let input: StreamConnectorsInput = parse_quote! {
            BinanceSpot => [UnknownKind]
        };
        assert!(input.validate().is_err());
    }
    
    #[test]
    fn test_validate_empty_kinds() {
         let input: StreamConnectorsInput = parse_quote! {
            BinanceSpot => []
         };
         assert!(input.validate().is_err());
    }
}

