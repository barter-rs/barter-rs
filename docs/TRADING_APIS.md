# Exchange Trading API Summary

This document summarises HTTP endpoints for order placement and cancellation across supported exchanges. These
endpoints were gathered from each venue's official documentation.

| Exchange | Order Placement Endpoint | Cancel Endpoint |
|----------|--------------------------|-----------------|
| Binance  | `POST /api/v3/order`     | `DELETE /api/v3/order` |
| Coinbase | `POST /orders`           | `DELETE /orders/<id>` |
| Kraken   | `POST /0/private/AddOrder` | `POST /0/private/CancelOrder` |
| OKX      | `POST /api/v5/trade/order` | `POST /api/v5/trade/cancel-order` |
| Kucoin   | `POST /api/v1/orders`    | `DELETE /api/v1/orders/<id>` |
| Bitget   | `POST /api/v2/order/place` | `POST /api/v2/order/cancel` |
| Gate.io  | `POST /api/v4/spot/orders` | `DELETE /api/v4/spot/orders/{order_id}` |
| Crypto.com | `POST /v2/private/create-order` | `POST /v2/private/cancel-order` |
| MEXC     | `POST /api/v3/order`     | `DELETE /api/v3/order` |
| Hyperliquid | `POST /api/v1/order`  | `POST /api/v1/cancel` |

All exchanges share the same high level behaviour: authenticated REST requests are issued with parameters describing
price, quantity and side. Cancellation typically requires the client order identifier or exchange generated order id.

Jackbot exposes a unified [`ExecutionClient`] trait that abstracts these operations so that strategies interact with
all venues in the same way.
