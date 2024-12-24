use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const AUDIT_MODULE_FILTER: &str = "barter::engine::audit=off";

/// Initialise default non-JSON `Barter` logging.
///
/// Note that this filters out duplicate logs produced by the `AuditManager` updating its replica
/// `EngineState`.
pub fn init_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy()
                .add_directive(
                    AUDIT_MODULE_FILTER
                        .parse()
                        .expect("audit module is not in expected directory"),
                ),
        )
        .with(tracing_subscriber::fmt::layer())
        .init()
}

/// Initialise default JSON `Barter` logging.
///
/// Note that this filters out duplicate logs produced by the `AuditManager` updating its replica
/// `EngineState`.
pub fn init_json_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy()
                .add_directive(
                    AUDIT_MODULE_FILTER
                        .parse()
                        .expect("audit module is not in expected directory"),
                ),
        )
        .with(tracing_subscriber::fmt::layer().json().flatten_event(true))
        .init()
}
