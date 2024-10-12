use crate::exchange::ExchangeId;
use crate::streams::reconnect;
use crate::streams::reconnect::{
    init_reconnecting_stream, ReconnectingStream, ReconnectionBackoffPolicy,
};
use crate::{
    error::DataError,
    event::MarketEvent,
    exchange::StreamSelector,
    instrument::InstrumentData,
    subscription::{Subscription, SubscriptionKind},
    Identifier, MarketStream,
};
use futures::{Stream, StreamExt};
use std::fmt::Display;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Default [`ReconnectionBackoffPolicy`] for a [`reconnecting`](`ReconnectingStream`) [`MarketStream`].
pub const DEFAULT_MARKET_STREAM_RECONNECTION_POLICY: ReconnectionBackoffPolicy =
    ReconnectionBackoffPolicy {
        backoff_ms_initial: 125,
        backoff_multiplier: 2,
        backoff_ms_max: 60000,
    };

/// Convenient type alias for a [`MarketEvent`] consumed via a
/// [`reconnecting`](`ReconnectingStream`) [`MarketStream`].
pub type MarketStreamEvent<InstrumentKey, Kind> =
    reconnect::Event<ExchangeId, Result<MarketEvent<InstrumentKey, Kind>, DataError>>;

/// Initialises a [`reconnecting`](`ReconnectingStream`) [`MarketStream`] using a collection of
/// [`Subscription`]s.
///
/// The provided [`ReconnectionBackoffPolicy`] dictates how the exponential backoff scales
/// between reconnections.
pub async fn init_reconnecting_market_stream<Exchange, Instrument, Kind>(
    subscriptions: Vec<Subscription<Exchange, Instrument, Kind>>,
    policy: ReconnectionBackoffPolicy,
) -> Result<impl Stream<Item = MarketStreamEvent<Instrument::Key, Kind::Event>>, DataError>
where
    Exchange: StreamSelector<Instrument, Kind>,
    Instrument: InstrumentData,
    Kind: SubscriptionKind + Display,
    Subscription<Exchange, Instrument, Kind>:
        Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
{
    // Determine ExchangeId associated with these Subscriptions
    let exchange = Exchange::ID;

    // Construct a MarketStream name for us in logging
    let stream_name = subscriptions
        .first()
        .map(|sub| {
            format!(
                "MarketStream({}_{}_{})",
                exchange,
                sub.instrument.kind(),
                sub.kind
            )
        })
        .ok_or(DataError::SubscriptionsEmpty)?;

    info!(
        %exchange,
        ?subscriptions,
        ?policy,
        %stream_name,
        "MarketStream with auto reconnect running"
    );

    let init_stream = move || {
        let subscriptions = subscriptions.clone();
        async move { Exchange::Stream::init(&subscriptions).await }
    };

    Ok(init_reconnecting_stream(init_stream)
        .await?
        .with_reconnect_backoff(policy, stream_name)
        .with_reconnection_events(exchange))
}

/// Initial duration that the [`consume`] function should wait after disconnecting before attempting
/// to re-initialise a [`MarketStream`]. This duration will increase exponentially as a result
/// of repeated disconnections with re-initialisation failures.
pub const STARTING_RECONNECT_BACKOFF_MS: u64 = 125;

/// Central [`MarketEvent<T>`](MarketEvent) consumer loop.
///
/// Initialises an exchange [`MarketStream`] using a collection of [`Subscription`]s. Consumed
/// events are distributed downstream via the `exchange_tx mpsc::UnboundedSender`. A re-connection
/// mechanism with an exponential backoff policy is utilised to ensure maximum up-time.
pub async fn consume<Exchange, Instrument, Kind>(
    subscriptions: Vec<Subscription<Exchange, Instrument, Kind>>,
    exchange_tx: mpsc::UnboundedSender<MarketEvent<Instrument::Key, Kind::Event>>,
) -> Result<(), DataError>
where
    Exchange: StreamSelector<Instrument, Kind>,
    Kind: SubscriptionKind,
    Instrument: InstrumentData,
    Subscription<Exchange, Instrument, Kind>:
        Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
{
    // Determine ExchangeId associated with these Subscriptions
    let exchange = Exchange::ID;

    info!(
        %exchange,
        ?subscriptions,
        policy = "retry connection with exponential backoff",
        "MarketStream consumer loop running",
    );

    // Consumer loop retry parameters
    let mut attempt: u32 = 0;
    let mut backoff_ms: u64 = STARTING_RECONNECT_BACKOFF_MS;

    'retry: loop {
        // Increment retry parameters at start of every iteration
        attempt += 1;
        backoff_ms *= 2;
        info!(%exchange, attempt, "attempting to initialise MarketStream");

        // Attempt to initialise MarketStream: if it fails on first attempt return DataError
        let mut stream = match Exchange::Stream::init(&subscriptions).await {
            Ok(stream) => {
                info!(%exchange, attempt, "successfully initialised MarketStream");
                attempt = 0;
                backoff_ms = STARTING_RECONNECT_BACKOFF_MS;
                stream
            }
            Err(error) => {
                error!(%exchange, attempt, ?error, "failed to initialise MarketStream");

                // Exit function if Stream::init failed the first attempt, else retry
                if attempt == 1 {
                    return Err(error);
                } else {
                    continue;
                }
            }
        };

        // Consume Result<MarketEvent<T>, DataError> from MarketStream
        while let Some(event_result) = stream.next().await {
            match event_result {
                // If Ok: send MarketEvent<T> to exchange receiver
                Ok(market_event) => {
                    if let Err(error) = exchange_tx.send(market_event) {
                        debug!(
                            payload = ?error.0,
                            why = "receiver dropped",
                            action = "shutting down Stream",
                            "failed to send Event<MarketData> to Exchange receiver"
                        );
                        break 'retry Ok(());
                    }
                }
                // If terminal DataError: break
                Err(error) if error.is_terminal() => {
                    error!(
                        %exchange,
                        %error,
                        action = "re-initialising Stream",
                        "consumed DataError from MarketStream",
                    );
                    break;
                }

                // If non-terminal DataError: log & continue
                Err(error) => {
                    warn!(
                        %exchange,
                        %error,
                        action = "skipping message",
                        "consumed DataError from MarketStream",
                    );
                    continue;
                }
            }
        }

        // If MarketStream ends unexpectedly, attempt re-connection after backoff_ms
        warn!(
            %exchange,
            backoff_ms,
            action = "attempt re-connection after backoff",
            "exchange MarketStream unexpectedly ended"
        );
        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
    }
}
