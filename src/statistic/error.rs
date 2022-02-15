use thiserror::Error;

/// All errors generated in the barter::statistic module.
#[derive(Error, Copy, Debug)]
pub enum StatisticError {
    #[error("Failed to build struct due to incomplete attributes provided")]
    BuilderIncomplete,

    #[error("Failed to build struct due to insufficient metrics provided")]
    BuilderNoMetricsProvided,
}
