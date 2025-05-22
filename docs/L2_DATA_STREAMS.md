# L2 Order Book Data Streams

This document provides details on the L2 order book data streams for all exchanges supported by JackBot Sensor.

## Overview

L2 order book data provides a detailed view of market depth with multiple price levels. Each exchange implements L2 data streams slightly differently, with variations in:

- Connection mechanisms
- Subscription methods
- Data structure and format
- Update frequency and depth
- Snapshot and delta update patterns

## Supported Exchanges

### Binance

#### Spot Market
- **WebSocket URL**: wss://stream.binance.com:9443/ws
- **Subscription Format**: `<symbol>@depth<levels>@<frequency>`
- **Data Format**: Provides both snapshots and incremental updates
- **Update Frequency**: 100ms (default), 1000ms available
- **Depth Levels**: 5, 10, 20 available for snapshots
- **Delta Updates**: Need to maintain local order book
- **Reference**: [Binance WebSocket API Documentation](https://binance-docs.github.io/apidocs/spot/en/#diff-depth-stream)

#### Futures Market
- **WebSocket URL**: wss://fstream.binance.com/ws
- **Subscription Format**: Similar to spot with futures-specific endpoints
- **Reference**: [Binance Futures WebSocket Documentation](https://binance-docs.github.io/apidocs/futures/en/#diff-book-depth-streams)

### Bitget

#### Spot Market
- **WebSocket URL**: wss://ws.bitget.com/spot/v1/stream
- **Subscription Method**: 
  ```json
  {
    "op": "subscribe",
    "args": [
      {
        "channel": "books",
        "instId": "<instrument_id>"
      }
    ]
  }
  ```
- **Data Format**: Provides snapshot followed by incremental updates
- **Depth Levels**: 100 price levels
- **Update Frequency**: Real-time
- **Reference**: [Bitget WebSocket API Documentation](https://bitgetlimited.github.io/apidoc/en/spot/#order-book-channel)

#### Futures Market
- **WebSocket URL**: wss://ws.bitget.com/mix/v1/stream
- **Subscription Method**: Similar to spot with futures-specific endpoints
- **Reference**: [Bitget Futures WebSocket Documentation](https://bitgetlimited.github.io/apidoc/en/mix/#order-book-channel)

### OKX

#### Spot and Futures Markets
- **WebSocket URL**: wss://ws.okx.com:8443/ws/v5/public
- **Subscription Method**:
  ```json
  {
    "op": "subscribe",
    "args": [
      {
        "channel": "books",
        "instId": "<instrument_id>"
      }
    ]
  }
  ```
- **Depth Options**: 
  - books: 400 levels, snapshot every 100ms
  - books5: 5 levels, snapshot every 100ms
  - books-l2-tbt: tick-by-tick, incremental updates
- **Checksum**: Provides checksum field for data verification
- **Reference**: [OKX WebSocket API Documentation](https://www.okx.com/docs-v5/en/#websocket-api-market-data-order-book-channel)

### Bybit

#### Spot and Futures Markets
- **WebSocket URL**: 
  - Spot: wss://stream.bybit.com/v5/public/spot
  - Futures: wss://stream.bybit.com/v5/public/linear
- **Subscription Method**:
  ```json
  {
    "op": "subscribe",
    "args": [
      "orderbook.50.<symbol>"
    ]
  }
  ```
- **Depth Options**: 1, 50, 200, 500 levels
- **Update Types**:
  - snapshot: Full orderbook
  - delta: Incremental updates
- **Reference**: [Bybit WebSocket API Documentation](https://bybit-exchange.github.io/docs/v5/websocket/public/orderbook)

### Merkle

#### Spot Market
- **WebSocket URL**: wss://api.merklex.io/ws
- **Subscription Method**:
  ```json
  {
    "op": "subscribe",
    "channel": "l2_updates",
    "markets": ["<market>"]
  }
  ```
- **Data Format**: Initial snapshot followed by incremental updates
- **Update Frequency**: Real-time
- **Reference**: [Merkle WebSocket API Documentation](https://docs.merklex.io/docs/ws-streams/orderbook)

## Implementation Requirements

For implementing L2 order book streams:

1. **Connection Management**:
   - Establish and maintain WebSocket connections
   - Handle reconnections and error cases
   - Manage authentication where required

2. **Message Processing**:
   - Parse incoming JSON/binary messages
   - Handle both snapshot and delta updates
   - Apply updates in sequence number order
   - Maintain local order book state

3. **Data Validation**:
   - Verify checksums where provided
   - Confirm sequence numbers for continuity
   - Request new snapshots if sequence mismatch detected

4. **Performance Considerations**:
   - Efficient data structures for order book representation
   - Fast update application
   - Memory optimization for deep order books

## Common Patterns

Despite exchange differences, most L2 implementations follow these patterns:

1. **Initial Snapshot**: Retrieve complete order book snapshot
2. **Delta Updates**: Process continuous incremental updates
3. **Sequence Numbers**: Track and verify update sequence
4. **Checksum Validation**: Verify data integrity where available
5. **Reconnection Logic**: Handle connection failures

## Testing Recommendations

When implementing L2 order book streams:

1. Verify correct handling of both snapshots and deltas
2. Confirm proper sequencing of updates
3. Test checksum validation
4. Simulate connection interruptions to verify recovery
5. Compare book state with exchange's public API for validation 