# Jackbot-Execution
Stream private account data from financial venues, and execute (live or mock) orders. Also provides
a feature rich MockExchange and MockExecutionClient to assist with backtesting and paper-trading.

**It is:**
* **Easy**: ExecutionClient trait provides a unified and simple language for interacting with exchanges.
* **Normalised**: Allow your strategy to communicate with every real or MockExchange using the same interface.
* **Extensible**: Jackbot-Execution is highly extensible, making it easy to contribute by adding new exchange integrations!

## Overview
High-performance and normalised trading interface capable of executing across many financial venues. Also provides
a feature rich simulated exchange to assist with backtesting and dry-trading. Communicate with an exchange by 
initialising it's associated `ExecutionClient` instance. 
