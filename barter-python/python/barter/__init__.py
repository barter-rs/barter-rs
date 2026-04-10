"""
Barter — Python bindings for the Barter algorithmic trading framework.

A high-performance Rust-powered library for:
- Real-time market data streaming from 12+ cryptocurrency exchanges
- Trading instrument management with O(1) indexed lookups
- Portfolio statistics (Sharpe, Sortino, Calmar, Drawdown, etc.)
- Backtesting with historical market data and mock execution
- Custom strategy and risk management callbacks

Quick Start:
    >>> from barter import ExchangeId, Instrument, IndexedInstruments

    # Define instruments
    >>> btc = Instrument.spot(ExchangeId.BINANCE_SPOT, "btc_usdt", "BTCUSDT", "btc", "usdt")
    >>> instruments = IndexedInstruments([btc])

    # Stream live market data
    >>> from barter import Subscription, build_market_stream
    >>> stream = build_market_stream([Subscription("binance_spot", "btc", "usdt")])
    >>> async for event in stream:
    ...     print(event)

    # Run a backtest with a custom strategy
    >>> from barter import run_backtest, OrderRequestOpen
    >>> def my_strategy(state):
    ...     orders = []
    ...     price = state.price(0)
    ...     if price and price < 50000:
    ...         orders.append(OrderRequestOpen(0, 0, "buy", price, 0.001))
    ...     return orders
    >>> summary = await run_backtest(config_json, data_json, strategy=my_strategy)
"""

from barter._barter import (
    # Account event types
    TradeFill,
    AccountEvent,
    PositionExited,
    # Instrument types
    Side,
    ExchangeId,
    Instrument,
    IndexedInstruments,
    # Execution types
    Balance,
    PublicTrade,
    # Order types
    OrderRequestOpen,
    OrderRequestCancel,
    # Engine state
    EngineState,
    # Order books
    OrderBook,
    OrderBookManager,
    build_order_book_manager,
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
    # Account event types
    "TradeFill",
    "AccountEvent",
    "PositionExited",
    # Instrument types
    "Side",
    "ExchangeId",
    "Instrument",
    "IndexedInstruments",
    # Execution types
    "Balance",
    "PublicTrade",
    # Order types
    "OrderRequestOpen",
    "OrderRequestCancel",
    # Engine state
    "EngineState",
    # Order books
    "OrderBook",
    "OrderBookManager",
    "build_order_book_manager",
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

__version__ = "0.2.0"
