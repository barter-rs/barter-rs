"""
Barter — Python bindings for the Barter algorithmic trading framework.

A high-performance Rust-powered library for:
- Real-time market data streaming from 12+ cryptocurrency exchanges
- Trading instrument management with O(1) indexed lookups
- Portfolio statistics (Sharpe, Sortino, Calmar, Drawdown, etc.)
- Backtesting with historical market data and mock execution

Quick Start:
    >>> from barter import ExchangeId, Instrument, IndexedInstruments

    # Define instruments
    >>> btc = Instrument.spot(ExchangeId.BINANCE_SPOT, "btc_usdt", "BTCUSDT", "btc", "usdt")
    >>> instruments = IndexedInstruments([btc])

    # Stream live market data
    >>> from barter import Subscription, build_market_stream
    >>> stream = await build_market_stream([Subscription("binance_spot", "btc", "usdt")])
    >>> async for event in stream:
    ...     print(event)
"""

from barter._barter import (
    # Instrument types
    Side,
    ExchangeId,
    Instrument,
    IndexedInstruments,
    # Execution types
    Balance,
    PublicTrade,
    # Market data
    MarketEvent,
    MarketDataStream,
    Subscription,
    build_market_stream,
    # Statistics
    TradingSummary,
    TradingSummaryGenerator,
    # Backtesting
    run_backtest,
)

__all__ = [
    # Instrument types
    "Side",
    "ExchangeId",
    "Instrument",
    "IndexedInstruments",
    # Execution types
    "Balance",
    "PublicTrade",
    # Market data
    "MarketEvent",
    "MarketDataStream",
    "Subscription",
    "build_market_stream",
    # Statistics
    "TradingSummary",
    "TradingSummaryGenerator",
    # Backtesting
    "run_backtest",
]

__version__ = "0.1.0"
