
# Barter-Data
A high-performance WebSocket integration library for streaming public market data from leading cryptocurrency 
exchanges - batteries included. It is:
* **Easy**: Barter-Data's simple StreamBuilder interface allows for easy & quick setup (see example below!).
* **Normalised**: Barter-Data's unified interface for consuming public WebSocket data means every Exchange returns a normalised data model.
* **Real-Time**: Barter-Data utilises real-time WebSocket integrations enabling the consumption of normalised tick-by-tick data.
* **Extensible**: Barter-Data is highly extensible, and therefore easy to contribute to with coding new integrations!

**See: [`Barter`], [`Barter-Integration`], [`Barter-Execution`] & [`Barter-Macro`]**

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/barter-data.svg
[crates-url]: https://crates.io/crates/barter-data

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://gitlab.com/open-source-keir/financial-modelling/trading/barter-data-rs/-/blob/main/LICENCE

[actions-badge]: https://gitlab.com/open-source-keir/financial-modelling/trading/barter-data-rs/badges/-/blob/main/pipeline.svg
[actions-url]: https://gitlab.com/open-source-keir/financial-modelling/trading/barter-data-rs/-/commits/main

[discord-badge]: https://img.shields.io/discord/910237311332151317.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/wE7RqhnQMV

[API Documentation] |
[Chat]

[`Barter`]: https://crates.io/crates/barter
[`Barter-Integration`]: https://crates.io/crates/barter-integration
[`Barter-Execution`]: https://crates.io/crates/barter-execution
[`Barter-Macro`]: https://crates.io/crates/barter-macro
[API Documentation]: https://docs.rs/barter-data/latest/barter_data
[Chat]: https://discord.gg/wE7RqhnQMV

## Overview
Barter-Data is a high-performance WebSocket integration library for streaming public market data from leading cryptocurrency 
exchanges. It presents an easy-to-use and extensible set of interfaces that can deliver normalised exchange data in real-time.

From a user perspective, the major component is the `StreamBuilder` structures that assists in initialising an 
arbitrary number of exchange `MarketStream`s using input `Subscription`s. Simply build your dream set of 
`MarketStreams` and `Barter-Data` will do the rest!

### Supported Exchange Subscriptions

|        Exchange         |         Constructor Code         |               InstrumentKinds               |                SubscriptionKinds                 |
|:-----------------------:|:--------------------------------:|:-------------------------------------------:|:------------------------------------------------:|
|     **BinanceSpot**     |     `BinanceSpot::default()`     |                    Spot                     | PublicTrades <br> OrderBooksL1 <br> OrderBooksL2 |
|  **BinanceFuturesUsd**  |  `BinanceFuturesUsd::default()`  |                  Perpetual                  | PublicTrades <br> OrderBooksL1 <br> OrderBooksL2 |
|      **Bitfinex**       |            `Bitfinex`            |                    Spot                     |                   PublicTrades                   |
|       **Bitmex**        |             `Bitmex`             |                  Perpetual                  |                   PublicTrades                   |
|      **BybitSpot**      |      `BybitSpot::default()`      |                    Spot                     |                   PublicTrades                   |
| **BybitPerpetualsUsd**  | `BybitPerpetualsUsd::default()`  |                  Perpetual                  |                   PublicTrades                   |
|      **Coinbase**       |            `Coinbase`            |                    Spot                     |                   PublicTrades                   |
|     **GateioSpot**      |     `GateioSpot::default()`      |                    Spot                     |                   PublicTrades                   |
|  **GateioFuturesUsd**   |  `GateioFuturesUsd::default()`   |                   Future                    |                   PublicTrades                   |
|  **GateioFuturesBtc**   |  `GateioFuturesBtc::default()`   |                   Future                    |                   PublicTrades                   |
| **GateioPerpetualsUsd** | `GateioPerpetualsUsd::default()` |                  Perpetual                  |                   PublicTrades                   |
| **GateioPerpetualsBtc** | `GateioPerpetualsBtc::default()` |                  Perpetual                  |                   PublicTrades                   |
|  **GateioOptionsBtc**   |    `GateioOptions::default()`    |                   Option                    |                   PublicTrades                   |
|       **Kraken**        |             `Kraken`             |                    Spot                     |          PublicTrades <br> OrderBooksL1          |
|         **Okx**         |              `Okx`               | Spot <br> Future <br> Perpetual <br> Option |                   PublicTrades                   |


## Examples
See barter-data-rs/examples for a more comprehensive selection of examples! 

### Multi Exchange Public Trades
```rust,no_run
use barter_data::{
    exchange::{
        binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
        bitmex::Bitmex,
        bybit::{futures::BybitPerpetualsUsd, spot::BybitSpot},
        coinbase::Coinbase,
        gateio::{
            option::GateioOptions,
            perpetual::{GateioPerpetualsBtc, GateioPerpetualsUsd},
            spot::GateioSpot,
        },
        okx::Okx,
    },
    streams::Streams,
    subscription::trade::PublicTrades,
};
use barter_integration::model::instrument::kind::{
    FutureContract, InstrumentKind, OptionContract, OptionExercise, OptionKind,
};
use chrono::{TimeZone, Utc};
use futures::StreamExt;
use tracing::info;

#[tokio::main]
async fn main() {
    // Initialise PublicTrades Streams for various exchanges
    // '--> each call to StreamBuilder::subscribe() creates a separate WebSocket connection
    let streams = Streams::<PublicTrades>::builder()
        .subscribe([
            (BinanceSpot::default(), "btc", "usdt", InstrumentKind::Spot, PublicTrades),
            (BinanceSpot::default(), "eth", "usdt", InstrumentKind::Spot, PublicTrades),
        ])
        .subscribe([
            (BinanceFuturesUsd::default(), "btc", "usdt", InstrumentKind::Perpetual, PublicTrades),
            (BinanceFuturesUsd::default(), "eth", "usdt", InstrumentKind::Perpetual, PublicTrades),
        ])
        .subscribe([
            (Coinbase, "btc", "usd", InstrumentKind::Spot, PublicTrades),
            (Coinbase, "eth", "usd", InstrumentKind::Spot, PublicTrades),
        ])
        .subscribe([
            (GateioSpot::default(), "btc", "usdt", InstrumentKind::Spot, PublicTrades),
        ])
        .subscribe([
            (GateioPerpetualsUsd::default(), "btc", "usdt", InstrumentKind::Perpetual, PublicTrades),
        ])
        .subscribe([
            (GateioPerpetualsBtc::default(), "btc", "usd", InstrumentKind::Perpetual, PublicTrades),
        ])
        .subscribe([
            (GateioOptions::default(), "btc", "usdt", InstrumentKind::Option(put_contract()), PublicTrades),
        ])
        .subscribe([
            (Okx, "btc", "usdt", InstrumentKind::Spot, PublicTrades),
            (Okx, "btc", "usdt", InstrumentKind::Perpetual, PublicTrades),
            (Okx, "btc", "usd", InstrumentKind::Future(future_contract()), PublicTrades),
            (Okx, "btc", "usd", InstrumentKind::Option(call_contract()), PublicTrades),
        ])
        .subscribe([
            (BybitSpot::default(), "btc", "usdt", InstrumentKind::Spot, PublicTrades),
            (BybitSpot::default(), "eth", "usdt", InstrumentKind::Spot, PublicTrades),
        ])
        .subscribe([
            (BybitPerpetualsUsd::default(), "btc", "usdt", InstrumentKind::Perpetual, PublicTrades),
        ])
        .subscribe([
            (Bitmex, "xbt", "usd", InstrumentKind::Perpetual, PublicTrades)
        ])
        .init()
        .await
        .unwrap();

    // Join all exchange PublicTrades streams into a single tokio_stream::StreamMap
    // Notes:
    //  - Use `streams.select(ExchangeId)` to interact with the individual exchange streams!
    //  - Use `streams.join()` to join all exchange streams into a single mpsc::UnboundedReceiver!
    let mut joined_stream = streams.join_map().await;

    while let Some((exchange, trade)) = joined_stream.next().await {
        info!("Exchange: {exchange}, MarketEvent<PublicTrade>: {trade:?}");
    }
}
```

## Getting Help
Firstly, see if the answer to your question can be found in the [API Documentation]. If the answer is not there, I'd be 
happy to help via [Chat] and try answer your question via Discord. 

## Contributing
Thanks in advance for helping to develop the Barter ecosystem! Please do get hesitate to get touch via the Discord 
[Chat] to discuss development, new features, and the future roadmap.

### Adding A New Exchange Connector
1. Add a new `Connector` trait implementation in src/exchange/<exchange_name>.mod.rs (eg/ see exchange::okx::Okx).
2. Follow on from "Adding A New Subscription Kind For An Existing Exchange Connector" below!

### Adding A New SubscriptionKind For An Existing Exchange Connector
1. Add a new `SubscriptionKind` trait implementation in src/subscription/<sub_kind_name>.rs (eg/ see subscription::trade::PublicTrades).
2. Define the `SubscriptionKind::Event` data model (eg/ see subscription::trade::PublicTrade).
3. Define the `MarketStream` type the exchange `Connector` will initialise for the new `SubscriptionKind`: <br>
   ie/ `impl StreamSelector<SubscriptionKind> for <ExistingExchangeConnector> { ... }`
4. Try to compile and follow the remaining steps!
5. Add a barter-data-rs/examples/<sub_kind_name>_streams.rs example in the standard format :)

## Related Projects
In addition to the Barter-Execution crate, the Barter project also maintains:
* [`Barter`]: High-performance, extensible & modular trading components with batteries-included. Contains a
  pre-built trading Engine that can serve as a live-trading or backtesting system.
* [`Barter-Integration`]: High-performance, low-level framework for composing flexible web integrations.
* [`Barter-Execution`]: High-performance WebSocket integration library for streaming public market data from leading
  cryptocurrency exchanges.
* [`Barter-Macro`]: Barter ecosystem macros.

## Roadmap
* Add support for more exchanges (easy to help with!)
* Add support for more subscription kinds (easy to help with!)

## Licence
This project is licensed under the [MIT license].

[MIT license]: https://gitlab.com/open-source-keir/financial-modelling/trading/barter-data-rs/-/blob/main/LICENSE

### Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Barter-Data by you, shall be licensed as MIT, without any additional
terms or conditions.
