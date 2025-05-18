# Barter-Data Exchange Integration Guide

This guide outlines the steps to integrate a new cryptocurrency exchange into the barter-data library, using the OneTrading implementation as a reference example.

## Overview

The barter-data library provides a unified interface for streaming market data from various cryptocurrency exchanges. Each exchange integration follows a similar pattern, allowing for normalized data structures across different sources.

## Required Components

For a new exchange integration, you'll need to implement the following components:

1. **Main Exchange Module** (`mod.rs`): Defines the main exchange struct and implements the `Connector` and `StreamSelector` traits.
2. **Channel Module** (`channel.rs`): Defines the exchange-specific channels (e.g., trades, orderbooks).
3. **Market Module** (`market.rs`): Handles market symbol formatting.
4. **Message Module** (`message.rs`): Defines the generic payload structure for deserializing exchange messages.
5. **Subscription Module** (`subscription.rs`): Handles subscription responses and validation.
6. **Trade Module** (`trade.rs`): Handles trade data structures and conversion.
7. **Orderbook Module** (`book/`): Handles orderbook data structures and conversion.

## Step-by-Step Implementation

### 1. Directory Structure

Create the directory structure for your exchange (e.g., `/exchange/myexchange/`):

```
barter-data/src/exchange/myexchange/
├── book/
│   ├── l1.rs        # Level 1 orderbook (top of book)
│   ├── l2.rs        # Level 2 orderbook (depth)
│   └── mod.rs       # Orderbook module exports
├── channel.rs       # Channel definitions
├── market.rs        # Market formatting
├── message.rs       # Message structure
├── mod.rs           # Main exchange implementation
├── subscription.rs  # Subscription handling
└── trade.rs         # Trade structures
```

### 2. Main Exchange Module (`mod.rs`)

This is the core of your exchange implementation. It defines the main exchange struct and implements the required traits:

```rust
use self::{
    channel::MyExchangeChannel, market::MyExchangeMarket, subscription::MyExchangeResponse, trade::MyExchangeTrade,
};
use crate::{
    ExchangeWsStream, NoInitialSnapshots,
    exchange::{Connector, ExchangeSub, PingInterval, StreamSelector},
    instrument::InstrumentData,
    subscriber::{WebSocketSubscriber, validator::WebSocketSubValidator},
    subscription::{
        Map, 
        book::{OrderBooksL1, OrderBooksL2},
        trade::PublicTrades,
    },
    transformer::stateless::StatelessTransformer,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{error::SocketError, protocol::websocket::WsMessage};
use barter_macro::{DeExchange, SerExchange};
use derive_more::Display;
use serde_json::json;
use std::time::Duration;
use url::Url;

// Import your modules
pub mod channel;
pub mod market;
pub mod message;
pub mod subscription;
pub mod trade;
pub mod book;

// Define the base URL for your exchange
pub const BASE_URL_MYEXCHANGE: &str = "wss://example.com/ws";

// Define the ping interval (if required by the exchange)
pub const PING_INTERVAL_MYEXCHANGE: Duration = Duration::from_secs(30);

// Define your exchange struct
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Debug,
    Default,
    Display,
    DeExchange,
    SerExchange,
)]
pub struct MyExchange;

// Implement the Connector trait
impl Connector for MyExchange {
    const ID: ExchangeId = ExchangeId::MyExchange;
    type Channel = MyExchangeChannel;
    type Market = MyExchangeMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = MyExchangeResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(BASE_URL_MYEXCHANGE).map_err(SocketError::UrlParse)
    }

    fn ping_interval() -> Option<PingInterval> {
        Some(PingInterval {
            interval: tokio::time::interval(PING_INTERVAL_MYEXCHANGE),
            ping: || {
                // Format the ping message according to the exchange's requirements
                WsMessage::text(
                    json!({
                        "type": "PING"
                    })
                    .to_string(),
                )
            },
        })
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        // Format the subscription request according to the exchange's API
        // Transform subscriptions into channel objects using Iterator API
        let channels = exchange_subs.into_iter()
            .map(|sub| {
                json!({
                    "name": sub.channel.as_ref(),
                    "instrument": sub.market.as_ref()
                })
            })
            .collect::<Vec<_>>();
        
        vec![WsMessage::text(
            json!({
                "type": "SUBSCRIBE",
                "channels": channels
            })
            .to_string(),
        )]
    }

    fn expected_responses<InstrumentKey>(_: &Map<InstrumentKey>) -> usize {
        // Return the number of responses expected from the exchange
        // Typically 1 for exchanges that send a single confirmation for all subscriptions
        1
    }
}

// Implement StreamSelector for PublicTrades
impl<Instrument> StreamSelector<Instrument, PublicTrades> for MyExchange
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream =
        ExchangeWsStream<StatelessTransformer<Self, Instrument::Key, PublicTrades, MyExchangeTrade>>;
}

// Implement StreamSelector for OrderBooksL1
impl<Instrument> StreamSelector<Instrument, OrderBooksL1> for MyExchange
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = ExchangeWsStream<
        StatelessTransformer<Self, Instrument::Key, OrderBooksL1, book::MyExchangeOrderBookL1Message>,
    >;
}

// Implement StreamSelector for OrderBooksL2
impl<Instrument> StreamSelector<Instrument, OrderBooksL2> for MyExchange
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = ExchangeWsStream<
        StatelessTransformer<Self, Instrument::Key, OrderBookEvent, book::MyExchangeOrderBookL2Message>,
    >;
}
```

### 3. Channel Module (`channel.rs`)

Define the exchange-specific channels and implement identifier traits:

```rust
use crate::{
    Identifier,
    exchange::myexchange::MyExchange,
    subscription::{
        Subscription,
        book::{OrderBooksL1, OrderBooksL2},
        trade::PublicTrades,
    },
};
use serde::Serialize;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct MyExchangeChannel(pub &'static str);

impl MyExchangeChannel {
    // Define channel constants based on the exchange's API
    pub const TRADES: Self = Self("PRICE_TICKS");
    pub const ORDER_BOOK_L1: Self = Self("BOOK_TICKER");
    pub const ORDER_BOOK_L2: Self = Self("ORDERBOOK");
}

// Implement Identifier trait for each subscription type
impl<Instrument> Identifier<MyExchangeChannel>
    for Subscription<MyExchange, Instrument, PublicTrades>
{
    fn id(&self) -> MyExchangeChannel {
        MyExchangeChannel::TRADES
    }
}

impl<Instrument> Identifier<MyExchangeChannel>
    for Subscription<MyExchange, Instrument, OrderBooksL1>
{
    fn id(&self) -> MyExchangeChannel {
        MyExchangeChannel::ORDER_BOOK_L1
    }
}

impl<Instrument> Identifier<MyExchangeChannel>
    for Subscription<MyExchange, Instrument, OrderBooksL2>
{
    fn id(&self) -> MyExchangeChannel {
        MyExchangeChannel::ORDER_BOOK_L2
    }
}

impl AsRef<str> for MyExchangeChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
```

### 4. Market Module (`market.rs`)

Handle market symbol formatting:

```rust
use crate::{
    Identifier, exchange::myexchange::MyExchange, instrument::MarketInstrumentData,
    subscription::Subscription,
};
use barter_instrument::{
    Keyed, asset::name::AssetNameInternal, instrument::market_data::MarketDataInstrument,
};
use serde::{Deserialize, Serialize};
use smol_str::{SmolStr, StrExt, format_smolstr};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct MyExchangeMarket(pub SmolStr);

// Implement Identifier for different instrument types
impl<Kind> Identifier<MyExchangeMarket>
    for Subscription<MyExchange, MarketDataInstrument, Kind>
{
    fn id(&self) -> MyExchangeMarket {
        myexchange_market(&self.instrument.base, &self.instrument.quote)
    }
}

impl<InstrumentKey, Kind> Identifier<MyExchangeMarket>
    for Subscription<MyExchange, Keyed<InstrumentKey, MarketDataInstrument>, Kind>
{
    fn id(&self) -> MyExchangeMarket {
        myexchange_market(&self.instrument.value.base, &self.instrument.value.quote)
    }
}

impl<InstrumentKey, Kind> Identifier<MyExchangeMarket>
    for Subscription<MyExchange, MarketInstrumentData<InstrumentKey>, Kind>
{
    fn id(&self) -> MyExchangeMarket {
        MyExchangeMarket(self.instrument.name_exchange.name().clone())
    }
}

impl AsRef<str> for MyExchangeMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// Format market symbol according to exchange's requirements
fn myexchange_market(base: &AssetNameInternal, quote: &AssetNameInternal) -> MyExchangeMarket {
    // Format according to exchange requirements (e.g., BTC_EUR)
    MyExchangeMarket(format_smolstr!("{}_{}", base, quote).to_uppercase_smolstr())
}
```

### 5. Message Module (`message.rs`)

Define the generic message structure for the exchange:

```rust
use std::fmt::Debug;

use crate::{Identifier, exchange::myexchange::channel::MyExchangeChannel};
use barter_integration::subscription::SubscriptionId;
use chrono::{DateTime, Utc};
use serde::{
    Deserialize, Serialize,
    de::{Error, Unexpected},
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct MyExchangePayload<T> {
    #[serde(rename = "type")]
    pub kind: String,
    
    #[serde(alias = "channel", deserialize_with = "de_message_subscription_id")]
    pub subscription_id: SubscriptionId,
    
    #[serde(
        alias = "time",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
    
    pub data: T,
}

// Helper function to deserialize subscription IDs
pub fn de_message_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    // Implement according to exchange's message format
    #[derive(Deserialize)]
    struct ChannelInfo<'a> {
        name: &'a str,
        instrument: &'a str,
    }

    let channel_info = ChannelInfo::deserialize(deserializer)?;
    
    // Map the channel name to internal channel constants
    let channel_name = match channel_info.name {
        "PRICE_TICKS" => MyExchangeChannel::TRADES.0,
        "BOOK_TICKER" => MyExchangeChannel::ORDER_BOOK_L1.0,
        "ORDERBOOK" => MyExchangeChannel::ORDER_BOOK_L2.0,
        _ => {
            return Err(Error::invalid_value(
                Unexpected::Str(channel_info.name),
                &"expected one of: PRICE_TICKS, BOOK_TICKER, ORDERBOOK",
            ))
        }
    };

    Ok(SubscriptionId::from(format!(
        "{}|{}",
        channel_name,
        channel_info.instrument
    )))
}

impl<T> Identifier<Option<SubscriptionId>> for MyExchangePayload<T> {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}
```

### 6. Subscription Module (`subscription.rs`)

Handle subscription responses and validation:

```rust
use barter_integration::{Validator, error::SocketError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct MyExchangeResponse {
    #[serde(rename = "type")]
    pub kind: MyExchangeResponseType,
    pub channels: Vec<MyExchangeChannel>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct MyExchangeChannel {
    pub name: String,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MyExchangeResponseType {
    Subscriptions,
    #[serde(alias = "ERROR")]
    Error,
    Pong,
}

// Implement the Validator trait to check subscription responses
impl Validator for MyExchangeResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match self.kind {
            MyExchangeResponseType::Subscriptions => {
                if !self.channels.is_empty() {
                    Ok(self)
                } else {
                    Err(SocketError::Subscribe(
                        "received empty channels in subscription response".to_owned(),
                    ))
                }
            }
            MyExchangeResponseType::Error => Err(SocketError::Subscribe(
                "received error subscription response".to_owned(),
            )),
            MyExchangeResponseType::Pong => Err(SocketError::Subscribe(
                "received pong message out of sequence".to_owned(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    // Add tests to verify response validation
}
```

### 7. Trade Module (`trade.rs`)

Handle trade data structures and conversion:

```rust
use crate::{
    event::{MarketEvent, MarketIter},
    exchange::myexchange::message::MyExchangePayload,
    subscription::trade::PublicTrade,
};
use barter_instrument::{Side, exchange::ExchangeId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Type alias for the trade message
pub type MyExchangeTrade = MyExchangePayload<MyExchangeTradeData>;

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct MyExchangeTradeData {
    pub instrument: String,
    
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,
    
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub amount: f64,
    
    #[serde(
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub timestamp: DateTime<Utc>,
    
    #[serde(deserialize_with = "de_side")]
    pub side: Side,
    
    pub id: String,
}

// Helper function to deserialize trade side
pub fn de_side<'de, D>(deserializer: D) -> Result<Side, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s = <&str>::deserialize(deserializer)?;
    match s {
        "BUY" => Ok(Side::Buy),
        "SELL" => Ok(Side::Sell),
        _ => Err(serde::de::Error::custom(format!("unknown side: {}", s))),
    }
}

// Implement conversion from exchange trade to Barter trade
impl<InstrumentKey: Clone> From<(ExchangeId, InstrumentKey, MyExchangeTrade)>
    for MarketIter<InstrumentKey, PublicTrade>
{
    fn from((exchange, instrument, trade): (ExchangeId, InstrumentKey, MyExchangeTrade)) -> Self {
        Self(
            vec![Ok(MarketEvent {
                time_exchange: trade.data.timestamp,
                time_received: Utc::now(),
                exchange,
                instrument: instrument.clone(),
                kind: PublicTrade {
                    id: trade.data.id,
                    price: trade.data.price,
                    amount: trade.data.amount,
                    side: trade.data.side,
                },
            })]
        )
    }
}

#[cfg(test)]
mod tests {
    // Add tests for trade deserialization and conversion
}
```

### 8. Orderbook Module

#### 8.1. `book/mod.rs`

```rust
mod l1;
mod l2;

pub use l1::*;
pub use l2::*;
```

#### 8.2. `book/l1.rs` (Level 1 Orderbook)

```rust
use crate::{
    books::Level,
    event::{MarketEvent, MarketIter},
    exchange::myexchange::message::MyExchangePayload,
    subscription::book::OrderBookL1,
};
use barter_instrument::exchange::ExchangeId;
use chrono::Utc;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// Type alias for L1 orderbook message
pub type MyExchangeOrderBookL1Message = MyExchangePayload<MyExchangeBookTickerData>;

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct MyExchangeBookTickerData {
    pub instrument: String,
    
    #[serde(rename = "bestBidPrice", deserialize_with = "barter_integration::de::de_str")]
    pub best_bid_price: f64,
    
    #[serde(rename = "bestBidAmount", deserialize_with = "barter_integration::de::de_str")]
    pub best_bid_amount: f64,
    
    #[serde(rename = "bestAskPrice", deserialize_with = "barter_integration::de::de_str")]
    pub best_ask_price: f64,
    
    #[serde(rename = "bestAskAmount", deserialize_with = "barter_integration::de::de_str")]
    pub best_ask_amount: f64,
    
    #[serde(
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub timestamp: chrono::DateTime<Utc>,
}

// Implement conversion from exchange orderbook to Barter orderbook
impl<InstrumentKey: Clone> From<(ExchangeId, InstrumentKey, MyExchangeOrderBookL1Message)>
    for MarketIter<InstrumentKey, OrderBookL1>
{
    fn from((exchange, instrument, message): (ExchangeId, InstrumentKey, MyExchangeOrderBookL1Message)) -> Self {
        Self(
            vec![Ok(MarketEvent {
                time_exchange: message.data.timestamp,
                time_received: Utc::now(),
                exchange,
                instrument: instrument.clone(),
                kind: OrderBookL1 {
                    last_update_time: message.data.timestamp,
                    best_bid: Some(Level {
                        price: Decimal::from_f64_retain(message.data.best_bid_price).unwrap_or_default(),
                        amount: Decimal::from_f64_retain(message.data.best_bid_amount).unwrap_or_default(),
                    }),
                    best_ask: Some(Level {
                        price: Decimal::from_f64_retain(message.data.best_ask_price).unwrap_or_default(),
                        amount: Decimal::from_f64_retain(message.data.best_ask_amount).unwrap_or_default(),
                    }),
                },
            })]
        )
    }
}

#[cfg(test)]
mod tests {
    // Add tests for orderbook deserialization and conversion
}
```

#### 8.3. `book/l2.rs` (Level 2 Orderbook)

```rust
use crate::{
    books::Level,
    books::OrderBook,
    event::{MarketEvent, MarketIter},
    exchange::myexchange::message::MyExchangePayload,
    subscription::book::OrderBookEvent,
};
use barter_instrument::exchange::ExchangeId;
use chrono::Utc;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// Type alias for L2 orderbook message
pub type MyExchangeOrderBookL2Message = MyExchangePayload<MyExchangeOrderBookData>;

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct MyExchangeOrderBookData {
    pub instrument: String,
    
    #[serde(deserialize_with = "de_orderbook_levels")]
    pub bids: Vec<Level>,
    
    #[serde(deserialize_with = "de_orderbook_levels")]
    pub asks: Vec<Level>,
    
    #[serde(
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub timestamp: chrono::DateTime<Utc>,
}

// Helper function to deserialize orderbook levels
fn de_orderbook_levels<'de, D>(deserializer: D) -> Result<Vec<Level>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let string_pairs: Vec<Vec<&str>> = Vec::deserialize(deserializer)?;
    
    string_pairs
        .into_iter()
        .map(|pair| {
            if pair.len() != 2 {
                return Err(serde::de::Error::custom(
                    "expected exactly 2 elements in price-amount pair",
                ));
            }
            
            let price = pair[0].parse::<f64>()
                .map_err(|_| serde::de::Error::custom("failed to parse price as float"))?;
                
            let amount = pair[1].parse::<f64>()
                .map_err(|_| serde::de::Error::custom("failed to parse amount as float"))?;
                
            Ok(Level {
                price: Decimal::from_f64_retain(price).unwrap_or_default(),
                amount: Decimal::from_f64_retain(amount).unwrap_or_default(),
            })
        })
        .collect()
}

// Implement conversion from exchange orderbook to Barter orderbook
impl<InstrumentKey: Clone> From<(ExchangeId, InstrumentKey, MyExchangeOrderBookL2Message)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from((exchange, instrument, message): (ExchangeId, InstrumentKey, MyExchangeOrderBookL2Message)) -> Self {
        // Create an OrderBook using the bids and asks
        let orderbook = OrderBook::new(
            0, // sequence number (using 0 for the initial snapshot)
            Some(message.data.timestamp), // time_engine
            message.data.bids,
            message.data.asks,
        );
        
        Self(
            vec![Ok(MarketEvent {
                time_exchange: message.data.timestamp,
                time_received: Utc::now(),
                exchange,
                instrument: instrument.clone(),
                kind: OrderBookEvent::Snapshot(orderbook),
            })]
        )
    }
}

#[cfg(test)]
mod tests {
    // Add tests for orderbook deserialization and conversion
}
```

### 9. Update Exchange ID

Make sure your exchange is added to the `ExchangeId` enum in the `barter-instrument` crate:

```rust
// barter-instrument/src/exchange.rs
pub enum ExchangeId {
    // ...existing exchanges
    MyExchange,
}

impl ExchangeId {
    pub fn as_str(&self) -> &'static str {
        match self {
            // ...existing matches
            ExchangeId::MyExchange => "my_exchange",
        }
    }
}
```

### 10. Register the Exchange Module

Add your exchange module to the `exchange/mod.rs` file:

```rust
// In barter-data/src/exchange/mod.rs
/// `MyExchange` [`Connector`] and [`StreamSelector`] implementations.
pub mod myexchange;
```

## Testing Your Implementation

Write comprehensive unit tests to verify:

1. Deserialization of messages
2. Validation of subscription responses
3. Conversion from exchange-specific types to Barter types

For example:

```rust
#[test]
fn test_myexchange_trade_deserialization() {
    let json = r#"{
        "type": "PRICE_TICK",
        "channel": {
            "name": "PRICE_TICKS",
            "instrument": "BTC_EUR"
        },
        "time": 1732051274299000000,
        "data": {
            "instrument": "BTC_EUR",
            "price": "51234.5",
            "amount": "0.00145",
            "timestamp": 1732051274298000000,
            "side": "BUY",
            "id": "trade_123456789"
        }
    }"#;

    let trade: Result<MyExchangeTrade, _> = serde_json::from_str(json);
    assert!(trade.is_ok());
    
    // Verify the parsed data
    let trade = trade.unwrap();
    assert_eq!(trade.data.price, 51234.5);
    assert_eq!(trade.data.amount, 0.00145);
    assert_eq!(trade.data.side, Side::Buy);
}
```

## Usage Example

Once your exchange is implemented, users can use it like this:

```rust
use barter_data::{
    exchange::myexchange::MyExchange,
    streams::builder::StreamBuilder,
    subscription::trade::PublicTrades,
};
use barter_instrument::instrument::market_data::MarketDataInstrument;

// Create a subscription to BTC-EUR trades on MyExchange
let subscription = StreamBuilder::new()
    .subscribe::<MyExchange, _, PublicTrades>(
        vec![MarketDataInstrument::from_base_quote_pair("BTC", "EUR")]
    );

// Process the trade stream
let mut stream = subscription.build().await.unwrap();
while let Some(event) = stream.next().await {
    // Handle the trade event
    println!("Trade: {:?}", event);
}
```

## Common Issues and Solutions

1. **Missing ExchangeId**: Make sure to add your exchange to the `ExchangeId` enum in `barter-instrument/src/exchange.rs`.

2. **Timestamp Format**: Many exchanges use different timestamp formats. Use the appropriate deserializer:
   - Milliseconds: `barter_integration::de::de_u64_epoch_ms_as_datetime_utc`
   - Seconds: `barter_integration::de::de_u64_epoch_as_datetime_utc`
   - Custom: Implement your own deserializer if needed

3. **Decimal Conversion**: When converting floating-point values to `Decimal`, use `Decimal::from_f64_retain()` with proper error handling.

4. **OrderBook Construction**: Make sure to use the correct `OrderBook::new()` constructor with all required parameters.

5. **Field Naming Conventions**: 
   - Always use snake_case for struct field names to follow Rust conventions
   - Use `#[serde(rename = "camelCaseFieldName")]` attributes to map between your snake_case fields and the API's camelCase/PascalCase field names
   - Example: `#[serde(rename = "bestBidPrice")] pub best_bid_price: f64`

## Best Practices

1. **Documentation**: Add documentation for all public types and functions, including examples of raw JSON payloads.

2. **Error Handling**: Add proper error handling and validation for all deserialized data.

3. **Testing**: Write comprehensive tests for all components, especially message deserialization and type conversion.

4. **Timestamps**: Handle exchange timestamps correctly, including correct timezone information.

5. **Decimal Precision**: Use the `Decimal` type for price and amount values to maintain precision.

6. **String Deserialization**: Always use `&str` instead of `String` when deserializing to avoid unnecessary allocations (e.g., `<&str>::deserialize(deserializer)?` instead of `String::deserialize(deserializer)?`).

7. **Prefer Iterator API**: Use Rust's Iterator API (`map`, `filter`, `collect`, etc.) instead of imperative loops for data transformation. This leads to more concise, readable, and often more efficient code.

8. **Follow Existing Patterns**: Always follow the established patterns in the codebase for consistency.

## Example Exchange References

For more complex examples, refer to these existing implementations:

- **Binance**: `barter-data/src/exchange/binance/`
- **Bybit**: `barter-data/src/exchange/bybit/`
- **OneTrading**: `barter-data/src/exchange/onetrading/`
- **OKX**: `barter-data/src/exchange/okx/`

## Contributing

After implementing your exchange:

1. Update this guide if you discover additional patterns or best practices
2. Submit a pull request with comprehensive test coverage
3. Include example usage in your documentation