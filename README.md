# barter-rs
Barter is an open-source Rust library containing **high-performance** & **modular** components for constructing both **live-trading & backtesting engines**.

## 1 Overview
The **main components** are **Data**, **Strategy**, **Portfolio**, **Execution** & **Statistic**.

Each components is stand-alone & de-coupled. Their behaviour is captured
in a set of communicative traits that define how each component responds to external events.

### 1.1 Data Handler, Strategy & Execution Handler
The **Data**, **Strategy** & **Execution components** are designed to be used by **one trading pair only** (eg/ ETH-USD on Binance). In order to **trade multiple pairs** on multiple exchanges, **many combinations** of these three components should be constructed and **run concurrently**.

#### 1.1.1 Data
- LiveCandlerHandler implementation provides a batteries included MarketEvent generator that utilises barter-data-rs unified WebSocket interface. This enables consumption of a normalised market data stream via the the underlying ExchangeClient.
- HistoricCandleHandler implementation included that allows a simulated MarketEvent generation based on a provided set of historical Candles.

### 1.2 Portfolio - Global State
The **Portfolio** component is designed in order to **manage infinite trading pairs simultaneously**,
creating a **centralised state machine** for managing trades across different asset classes,
exchanges, and trading pairs.

### 1.3 Statistics
The **Statistic** component contains metrics used to analyse trading session performance.
One-pass dispersion algorithms analyse each closed Position and efficiently update
a PnL Return Summary. This summary, in conjunction with the closed Position, is used to calculate
key metrics such as Sharpe Ratio, Calmar Ratio and Max Drawdown. All metrics can be updated on
the fly and used by the Portfolio's allocation & risk management.

## 2 High-Level Data Architecture Example Using Barter Components
![](barter_components.png "Example Barter Components Data Architecture")