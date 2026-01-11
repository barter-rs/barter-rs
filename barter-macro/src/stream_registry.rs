use std::collections::HashSet;
use syn::parse::{Parse, ParseStream};
use syn::{bracketed, Ident, Result, Token, Error};
use syn::punctuated::Punctuated;

/// A single connector registration entry.
///
/// Example: `BinanceSpot => [PublicTrades, OrderBooksL1]`
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

/// The complete macro input.
///
/// Example:
/// ```rust,ignore
/// BinanceSpot => [PublicTrades, OrderBooksL1],
/// Coinbase => [PublicTrades],
/// ```
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

