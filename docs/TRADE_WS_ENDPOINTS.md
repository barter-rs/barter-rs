# Trade WebSocket Endpoints

This document summarises WebSocket endpoints for real-time trade streams across all exchanges supported by Jackbot. The endpoints below were gathered from each venue's official documentation.

## Spot Markets
| Exchange | WebSocket Endpoint |
|----------|-------------------|
| Binance Spot | `wss://stream.binance.com:9443/ws` |
| Bitget Spot | `wss://ws.bitget.com/spot/v1/stream` |
| Bybit Spot | `wss://stream.bybit.com/v5/public/spot` |
| Coinbase | `wss://ws-feed.exchange.coinbase.com` |
| Kraken Spot | `wss://ws.kraken.com/` |
| Kucoin Spot | `wss://ws-api.kucoin.com/endpoint` |
| OKX Spot | `wss://ws.okx.com:8443/ws/v5/public` |
| Gate.io Spot | `wss://api.gateio.ws/ws/v4/` |
| Crypto.com Spot | `wss://stream.crypto.com/v2/market` |
| MEXC Spot | `wss://wbs.mexc.com/ws` |
| Hyperliquid Spot | `wss://api.hyperliquid.xyz/ws` |

## Futures Markets
| Exchange | WebSocket Endpoint |
|----------|-------------------|
| Binance Futures | `wss://fstream.binance.com/ws` |
| Bitget Futures | `wss://ws.bitget.com/mix/v1/stream` |
| Bybit Futures | `wss://stream.bybit.com/v5/public/linear` |
| Kraken Futures | `wss://futures.kraken.com/ws/v1` |
| Kucoin Futures | `wss://ws-api-futures.kucoin.com/endpoint` |
| OKX Futures | `wss://ws.okx.com:8443/ws/v5/public` |
| Gate.io Futures | `wss://fx-ws.gateio.ws/v4/ws/` |
| Crypto.com Futures | `wss://deriv-stream.crypto.com/v1/market` |
| MEXC Futures | `wss://contract.mexc.com/ws` |
| Hyperliquid Futures | `wss://api.hyperliquid.xyz/ws` |

## Usage

Trade listeners in Jackbot connect to these endpoints using the appropriate subscription format for each exchange. Consult each exchange's documentation for the exact JSON payloads required for trade subscriptions.
