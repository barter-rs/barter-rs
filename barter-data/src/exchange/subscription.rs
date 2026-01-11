use crate::{Identifier, subscription::Subscription};
use barter_integration::subscription::SubscriptionId;
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Market Identifier Macros
// ---------------------------------------------------------------------------

/// Generates `Identifier<$Market>` implementations for exchange types that use a simple
/// base/quote concatenation pattern.
///
/// This macro reduces boilerplate for exchanges where the market identifier is derived from
/// the instrument's base and quote assets using a helper function.
///
/// # Variants
///
/// ## With Server Type Parameter
/// For exchanges like Binance, Bybit that have a server type parameter:
/// ```ignore
/// impl_market_identifier!(
///     Exchange<Server> => Market,
///     market_fn
/// );
/// ```
///
/// ## Without Server Type Parameter  
/// For exchanges like Kraken, Coinbase without a server type:
/// ```ignore
/// impl_market_identifier!(
///     Exchange => Market,
///     market_fn
/// );
/// ```
///
/// # Generated Implementations
/// For each variant, generates three impls:
/// 1. `Subscription<Exchange, MarketDataInstrument, Kind>` - uses the market_fn helper
/// 2. `Subscription<Exchange, Keyed<K, MarketDataInstrument>, Kind>` - uses the market_fn helper
/// 3. `Subscription<Exchange, MarketInstrumentData<K>, Kind>` - uses name_exchange directly
#[macro_export]
macro_rules! impl_market_identifier {
    // Variant with Server type parameter (e.g., Binance<Server>, Bybit<Server>, Gateio<Server>)
    ($Exchange:ident<$Server:ident> => $Market:ident, $market_fn:path) => {
        impl<$Server, Kind> $crate::Identifier<$Market>
            for $crate::subscription::Subscription<
                $Exchange<$Server>,
                ::barter_instrument::instrument::market_data::MarketDataInstrument,
                Kind,
            >
        {
            fn id(&self) -> $Market {
                $market_fn(&self.instrument.base, &self.instrument.quote)
            }
        }

        impl<$Server, InstrumentKey, Kind> $crate::Identifier<$Market>
            for $crate::subscription::Subscription<
                $Exchange<$Server>,
                ::barter_instrument::Keyed<
                    InstrumentKey,
                    ::barter_instrument::instrument::market_data::MarketDataInstrument,
                >,
                Kind,
            >
        {
            fn id(&self) -> $Market {
                $market_fn(
                    &self.instrument.as_ref().base,
                    &self.instrument.as_ref().quote,
                )
            }
        }

        impl<$Server, InstrumentKey, Kind> $crate::Identifier<$Market>
            for $crate::subscription::Subscription<
                $Exchange<$Server>,
                $crate::instrument::MarketInstrumentData<InstrumentKey>,
                Kind,
            >
        {
            fn id(&self) -> $Market {
                $Market(self.instrument.name_exchange.name().clone())
            }
        }
    };

    // Variant without Server type parameter (e.g., Kraken, Coinbase, Bitfinex, Bitmex)
    ($Exchange:ty => $Market:ident, $market_fn:path) => {
        impl<Kind> $crate::Identifier<$Market>
            for $crate::subscription::Subscription<
                $Exchange,
                ::barter_instrument::instrument::market_data::MarketDataInstrument,
                Kind,
            >
        {
            fn id(&self) -> $Market {
                $market_fn(&self.instrument.base, &self.instrument.quote)
            }
        }

        impl<InstrumentKey, Kind> $crate::Identifier<$Market>
            for $crate::subscription::Subscription<
                $Exchange,
                ::barter_instrument::Keyed<
                    InstrumentKey,
                    ::barter_instrument::instrument::market_data::MarketDataInstrument,
                >,
                Kind,
            >
        {
            fn id(&self) -> $Market {
                $market_fn(&self.instrument.value.base, &self.instrument.value.quote)
            }
        }

        impl<InstrumentKey, Kind> $crate::Identifier<$Market>
            for $crate::subscription::Subscription<
                $Exchange,
                $crate::instrument::MarketInstrumentData<InstrumentKey>,
                Kind,
            >
        {
            fn id(&self) -> $Market {
                $Market(self.instrument.name_exchange.name().clone())
            }
        }
    };
}

/// Generates `Identifier<$Market>` implementations for exchanges that derive the market
/// from the full [`MarketDataInstrument`] (including its kind), rather than just base/quote.
///
/// Used for exchanges like Okx and Gateio where the market string depends on the
/// instrument kind (Spot, Perpetual, Future, Option).
///
/// # Variants
///
/// ## With Server Type Parameter
/// ```ignore
/// impl_market_identifier_for_instrument!(
///     Exchange<Server> => Market,
///     market_fn
/// );
/// ```
///
/// ## Without Server Type Parameter
/// ```ignore
/// impl_market_identifier_for_instrument!(
///     Exchange => Market,
///     market_fn
/// );
/// ```
#[macro_export]
macro_rules! impl_market_identifier_for_instrument {
    // Variant with Server type parameter (e.g., Gateio<Server>)
    ($Exchange:ident<$Server:ident> => $Market:ident, $market_fn:path) => {
        impl<$Server, Kind> $crate::Identifier<$Market>
            for $crate::subscription::Subscription<
                $Exchange<$Server>,
                ::barter_instrument::instrument::market_data::MarketDataInstrument,
                Kind,
            >
        {
            fn id(&self) -> $Market {
                $market_fn(&self.instrument)
            }
        }

        impl<$Server, InstrumentKey, Kind> $crate::Identifier<$Market>
            for $crate::subscription::Subscription<
                $Exchange<$Server>,
                ::barter_instrument::Keyed<
                    InstrumentKey,
                    ::barter_instrument::instrument::market_data::MarketDataInstrument,
                >,
                Kind,
            >
        {
            fn id(&self) -> $Market {
                $market_fn(&self.instrument.value)
            }
        }

        impl<$Server, InstrumentKey, Kind> $crate::Identifier<$Market>
            for $crate::subscription::Subscription<
                $Exchange<$Server>,
                $crate::instrument::MarketInstrumentData<InstrumentKey>,
                Kind,
            >
        {
            fn id(&self) -> $Market {
                $Market(self.instrument.name_exchange.name().clone())
            }
        }
    };

    // Variant without Server type parameter (e.g., Okx)
    ($Exchange:ty => $Market:ident, $market_fn:path) => {
        impl<Kind> $crate::Identifier<$Market>
            for $crate::subscription::Subscription<
                $Exchange,
                ::barter_instrument::instrument::market_data::MarketDataInstrument,
                Kind,
            >
        {
            fn id(&self) -> $Market {
                $market_fn(&self.instrument)
            }
        }

        impl<InstrumentKey, Kind> $crate::Identifier<$Market>
            for $crate::subscription::Subscription<
                $Exchange,
                ::barter_instrument::Keyed<
                    InstrumentKey,
                    ::barter_instrument::instrument::market_data::MarketDataInstrument,
                >,
                Kind,
            >
        {
            fn id(&self) -> $Market {
                $market_fn(&self.instrument.value)
            }
        }

        impl<InstrumentKey, Kind> $crate::Identifier<$Market>
            for $crate::subscription::Subscription<
                $Exchange,
                $crate::instrument::MarketInstrumentData<InstrumentKey>,
                Kind,
            >
        {
            fn id(&self) -> $Market {
                $Market(self.instrument.name_exchange.name().clone())
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Channel Identifier Macros
// ---------------------------------------------------------------------------

/// Generates `Identifier<$Channel>` implementations that return a constant channel value
/// for a given subscription kind.
///
/// # Variants
///
/// ## With Server Type Parameter
/// For exchanges like Binance, Bybit that have a server type parameter:
/// ```ignore
/// impl_channel_identifier!(
///     Exchange<Server>, Instrument => Channel,
///     SubscriptionKind => Channel::CONSTANT
/// );
/// ```
///
/// ## Without Server Type Parameter
/// For exchanges like Kraken, Coinbase without a server type:
/// ```ignore
/// impl_channel_identifier!(
///     Exchange, Instrument => Channel,
///     SubscriptionKind => Channel::CONSTANT
/// );
/// ```
#[macro_export]
macro_rules! impl_channel_identifier {
    // Variant with Server type parameter
    ($Exchange:ident<$Server:ident>, $Instrument:ident => $Channel:ident, $SubKind:ty => $channel_const:expr) => {
        impl<$Server, $Instrument> $crate::Identifier<$Channel>
            for $crate::subscription::Subscription<$Exchange<$Server>, $Instrument, $SubKind>
        {
            fn id(&self) -> $Channel {
                $channel_const
            }
        }
    };

    // Variant without Server type parameter
    ($Exchange:ty, $Instrument:ident => $Channel:ident, $SubKind:ty => $channel_const:expr) => {
        impl<$Instrument> $crate::Identifier<$Channel>
            for $crate::subscription::Subscription<$Exchange, $Instrument, $SubKind>
        {
            fn id(&self) -> $Channel {
                $channel_const
            }
        }
    };
}

/// Defines an exchange specific market and channel combination used by an exchange
/// [`Connector`](super::Connector) to build the
/// [`WsMessage`](barter_integration::protocol::websocket::WsMessage) subscription payloads to
/// send to the exchange server.
///
/// ### Examples
/// #### Binance OrderBooksL2
/// ```json
/// ExchangeSub {
///     channel: BinanceChannel("@depth@100ms"),
///     market: BinanceMarket("btcusdt"),
/// }
/// ```
/// #### Kraken PublicTrades
/// ```json
/// ExchangeSub {
///     channel: KrakenChannel("trade"),
///     market: KrakenChannel("BTC/USDT")
/// }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize)]
pub struct ExchangeSub<Channel, Market> {
    /// Type that defines how to translate a Barter [`Subscription`] into an exchange specific
    /// channel to be subscribed to.
    ///
    /// ### Examples
    /// - [`BinanceChannel("@depth@100ms")`](super::binance::channel::BinanceChannel)
    /// - [`KrakenChannel("trade")`](super::kraken::channel::KrakenChannel)
    pub channel: Channel,

    /// Type that defines how to translate a Barter [`Subscription`] into an exchange specific
    /// market that can be subscribed to.
    ///
    /// ### Examples
    /// - [`BinanceMarket("btcusdt")`](super::binance::market::BinanceMarket)
    /// - [`KrakenMarket("BTC/USDT")`](super::kraken::market::KrakenMarket)
    pub market: Market,
}

impl<Channel, Market> Identifier<SubscriptionId> for ExchangeSub<Channel, Market>
where
    Channel: AsRef<str>,
    Market: AsRef<str>,
{
    fn id(&self) -> SubscriptionId {
        SubscriptionId::from(format!(
            "{}|{}",
            self.channel.as_ref(),
            self.market.as_ref()
        ))
    }
}

impl<Channel, Market> ExchangeSub<Channel, Market>
where
    Channel: AsRef<str>,
    Market: AsRef<str>,
{
    /// Construct a new exchange specific [`Self`] with the Barter [`Subscription`] provided.
    pub fn new<Exchange, Instrument, Kind>(sub: &Subscription<Exchange, Instrument, Kind>) -> Self
    where
        Subscription<Exchange, Instrument, Kind>: Identifier<Channel> + Identifier<Market>,
    {
        Self {
            channel: sub.id(),
            market: sub.id(),
        }
    }
}

impl<Channel, Market> From<(Channel, Market)> for ExchangeSub<Channel, Market>
where
    Channel: AsRef<str>,
    Market: AsRef<str>,
{
    fn from((channel, market): (Channel, Market)) -> Self {
        Self { channel, market }
    }
}
