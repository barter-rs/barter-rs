# Exchange Trading API Summary

This document summarises HTTP endpoints for order placement and cancellation across supported exchanges. These endpoints were gathered from each venue's official documentation.

## Spot REST Endpoints
| Exchange | Order Placement Endpoint | Cancel Endpoint |
|----------|--------------------------|-----------------|
| Binance Spot | `POST /api/v3/order` | `DELETE /api/v3/order` |
| Coinbase | `POST /orders` | `DELETE /orders/<id>` |
| Kraken Spot | `POST /0/private/AddOrder` | `POST /0/private/CancelOrder` |
| OKX Spot | `POST /api/v5/trade/order` | `POST /api/v5/trade/cancel-order` |
| Kucoin Spot | `POST /api/v1/orders` | `DELETE /api/v1/orders/<id>` |
| Bitget Spot | `POST /api/v2/order/place` | `POST /api/v2/order/cancel` |
| Gate.io Spot | `POST /api/v4/spot/orders` | `DELETE /api/v4/spot/orders/{order_id}` |
| Crypto.com Spot | `POST /v2/private/create-order` | `POST /v2/private/cancel-order` |
| MEXC Spot | `POST /api/v3/order` | `DELETE /api/v3/order` |
| Hyperliquid | `POST /api/v1/order` | `POST /api/v1/cancel` |

## Futures REST Endpoints
| Exchange | Order Placement Endpoint | Cancel Endpoint |
|----------|--------------------------|-----------------|
| Binance Futures | `POST /fapi/v1/order` | `DELETE /fapi/v1/order` |
| Bybit Futures | `POST /v5/order/create` | `POST /v5/order/cancel` |
| Bitget Futures | `POST /api/mix/v1/order/place` | `POST /api/mix/v1/order/cancel` |
| Gate.io Futures | `POST /api/v4/futures/{settle}/orders` | `DELETE /api/v4/futures/{settle}/orders/{order_id}` |
| Kraken Futures | `POST /api/v3/sendorder` | `POST /api/v3/cancelorder` |
| Kucoin Futures | `POST /api/v1/orders` | `DELETE /api/v1/orders/<id>` |
| OKX Futures | `POST /api/v5/trade/order` | `POST /api/v5/trade/cancel-order` |
| Crypto.com Futures | `POST /v2/private/create-order` | `POST /v2/private/cancel-order` |
| MEXC Futures | `POST /api/v1/private/order` | `DELETE /api/v1/private/order` |

All exchanges share the same high level behaviour: authenticated REST requests are issued with parameters describing price, quantity and side. Cancellation typically requires the client order identifier or exchange generated order id.

Jackbot exposes a unified [`ExecutionClient`] trait that abstracts these operations so that strategies interact with all venues in the same way.
