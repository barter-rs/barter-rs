use thiserror::Error;

/// All errors generated in the barter::statistic module.
#[derive(Error, Copy, Clone, Debug)]
pub enum StatisticError {
    #[error("Failed to build struct due to missing attributes: {0}")]
    BuilderIncomplete(&'static str),

    #[error("Failed to build struct due to insufficient metrics provided")]
    BuilderNoMetricsProvided,
}
