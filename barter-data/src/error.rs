use crate::{exchange::ExchangeId, subscription::SubKind};
use barter_integration::error::SocketError;
use thiserror::Error;

/// All errors generated in `barter-data`.
#[derive(Debug, Error)]
pub enum DataError {
    #[error("SocketError: {0}")]
    Socket(#[from] SocketError),

    #[error("unsupported dynamic Subscription for exchange: {exchange}, kind: {sub_kind}")]
    Unsupported {
        exchange: ExchangeId,
        sub_kind: SubKind,
    },

    #[error(
        "\
        InvalidSequence: first_update_id {first_update_id} does not follow on from the \
        prev_last_update_id {prev_last_update_id} \
    "
    )]
    InvalidSequence {
        prev_last_update_id: u64,
        first_update_id: u64,
    },
}

impl DataError {
    /// Determine if an error requires a [`MarketStream`](super::MarketStream) to re-initialise.
    #[allow(clippy::match_like_matches_macro)]
    pub fn is_terminal(&self) -> bool {
        match self {
            DataError::InvalidSequence { .. } => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_error_is_terminal() {
        struct TestCase {
            input: DataError,
            expected: bool,
        }

        let tests = vec![
            TestCase {
                // TC0: is terminal w/ DataError::InvalidSequence
                input: DataError::InvalidSequence {
                    prev_last_update_id: 0,
                    first_update_id: 0,
                },
                expected: true,
            },
            TestCase {
                // TC1: is not terminal w/ DataError::Socket
                input: DataError::Socket(SocketError::Sink),
                expected: false,
            },
        ];

        for (index, test) in tests.into_iter().enumerate() {
            let actual = test.input.is_terminal();
            assert_eq!(actual, test.expected, "TC{} failed", index);
        }
    }
}
