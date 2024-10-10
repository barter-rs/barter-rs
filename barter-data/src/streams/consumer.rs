use crate::exchange::ExchangeId;
use crate::streams::reconnect;
use crate::streams::reconnect::stream::ReconnectingStream;
use crate::streams::reconnect::stream::{init_reconnecting_stream, ReconnectionBackoffPolicy};
use crate::{
    error::DataError,
    event::MarketEvent,
    exchange::StreamSelector,
    instrument::InstrumentData,
    subscription::{Subscription, SubscriptionKind},
    Identifier, MarketStream,
};
use futures::Stream;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Default [`ReconnectionBackoffPolicy`] for a [`reconnecting`](`ReconnectingStream`) [`MarketStream`].
pub const STREAM_RECONNECTION_POLICY: ReconnectionBackoffPolicy = ReconnectionBackoffPolicy {
    backoff_ms_initial: 125,
    backoff_multiplier: 2,
    backoff_ms_max: 60000,
};

/// Convenient type alias for a [`MarketEvent`] [`Result`] consumed via a
/// [`reconnecting`](`ReconnectingStream`) [`MarketStream`].
pub type MarketStreamResult<InstrumentKey, Kind> =
    reconnect::Event<ExchangeId, Result<MarketEvent<InstrumentKey, Kind>, DataError>>;

/// Convenient type alias for a [`MarketEvent`] consumed via a
/// [`reconnecting`](`ReconnectingStream`) [`MarketStream`].
pub type MarketStreamEvent<InstrumentKey, Kind> =
    reconnect::Event<ExchangeId, MarketEvent<InstrumentKey, Kind>>;

/// Initialises a [`reconnecting`](`ReconnectingStream`) [`MarketStream`] using a collection of
/// [`Subscription`]s.
///
/// The provided [`ReconnectionBackoffPolicy`] dictates how the exponential backoff scales
/// between reconnections.
pub async fn init_market_stream<Exchange, Instrument, Kind>(
    policy: ReconnectionBackoffPolicy,
    subscriptions: Vec<Subscription<Exchange, Instrument, Kind>>,
) -> Result<impl Stream<Item = MarketStreamResult<Instrument::Key, Kind::Event>>, DataError>
where
    Exchange: StreamSelector<Instrument, Kind>,
    Instrument: InstrumentData,
    Kind: SubscriptionKind,
    Subscription<Exchange, Instrument, Kind>:
        Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
{
    // Determine ExchangeId associated with these Subscriptions
    let exchange = Exchange::ID;

    // Determine StreamKey for use in logging
    let stream_key = subscriptions
        .first()
        .map(|sub| StreamKey {
            exchange,
            kind: sub.kind.as_str(),
        })
        .ok_or(DataError::SubscriptionsEmpty)?;

    info!(
        %exchange,
        ?subscriptions,
        ?policy,
        ?stream_key,
        "MarketStream with auto reconnect running"
    );

    Ok(init_reconnecting_stream(move || {
        let subscriptions = subscriptions.clone();
        async move { Exchange::Stream::init::<Exchange::SnapFetcher>(&subscriptions).await }
    })
    .await?
    .with_reconnect_backoff(policy, stream_key)
    .with_termination_on_error(|error| error.is_terminal(), stream_key)
    .with_reconnection_events(exchange))
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct StreamKey<Kind = &'static str> {
    pub exchange: ExchangeId,
    pub kind: Kind,
}

impl std::fmt::Debug for StreamKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "market_stream-{}-{}", self.exchange, self.kind)
    }
}
