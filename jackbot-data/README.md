# Jackbot-Data
A high-performance WebSocket integration library for streaming public market data from leading cryptocurrency 
exchanges - batteries included. It is:
* **Easy**: Jackbot-Data's simple StreamBuilder interface allows for easy & quick setup (see example below!).
* **Normalised**: Jackbot-Data's unified interface for consuming public WebSocket data means every Exchange returns a normalised data model.
* **Real-Time**: Jackbot-Data utilises real-time WebSocket integrations enabling the consumption of normalised tick-by-tick data.
* **Extensible**: Jackbot-Data is highly extensible, and therefore easy to contribute to with coding new integrations!

## Overview
Jackbot-Data is a high-performance WebSocket integration library for streaming public market data from leading cryptocurrency 
exchanges. It presents an easy-to-use and extensible set of interfaces that can deliver normalised exchange data in real-time.

From a user perspective, the major component is the `StreamBuilder` structures that assists in initialising an 
arbitrary number of exchange `MarketStream`s using input `Subscription`s. Simply build your dream set of 
`MarketStreams` and `Jackbot-Data` will do the rest!

### Adding A New Exchange Connector
1. Add a new `Connector` trait implementation in src/exchange/<exchange_name>.mod.rs (eg/ see exchange::okx::Okx).
2. Follow on from "Adding A New Subscription Kind For An Existing Exchange Connector" below!

### Adding A New SubscriptionKind For An Existing Exchange Connector
1. Add a new `SubscriptionKind` trait implementation in src/subscription/<sub_kind_name>.rs (eg/ see subscription::trade::PublicTrades).
2. Define the `SubscriptionKind::Event` data model (eg/ see subscription::trade::PublicTrade).
3. Define the `MarketStream` type the exchange `Connector` will initialise for the new `SubscriptionKind`: <br>
   ie/ `impl StreamSelector<SubscriptionKind> for <ExistingExchangeConnector> { ... }`
4. Try to compile and follow the remaining steps!
5. Add a jackbot-data/examples/<sub_kind_name>_streams.rs example in the standard format