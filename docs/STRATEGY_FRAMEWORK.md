# Jackbot Strategy Framework

This document provides a brief overview of the strategy utilities added to Jackbot.

## Strategy Abstraction

`strategy::framework::Strategy` combines the existing `AlgoStrategy` and
`ClosePositionsStrategy` traits. Strategies implementing this trait expose an
`id` method for registry lookups.

The standalone `jackbot-strategy` crate offers a lightweight `Strategy` trait
with `on_start`, `on_event` and `on_stop` hooks for simple event driven
strategies.

## Strategy Registry

`strategy::registry::StrategyRegistry` offers a lightweight container for
storing strategies keyed by `StrategyId`. It provides simple `register`, `get`
and `remove` helpers so strategies can be discovered dynamically.

## Technical Analysis Library

The `technical` module implements a few basic indicators used across example
strategies:

- `sma` – Simple moving average
- `ema` – Exponential moving average
- `rsi` – Relative strength index

These functions operate on `f64` slices and return a `Vec<f64>` containing the
indicator values.

## Machine Learning Model Integration

The `ml` module provides a minimal `Model` trait and a serialisable
`LinearModel`. Models can be loaded from JSON and used to generate predictions
inside strategies.

## Configuration Management

`strategy::config::StrategyConfig` is a simple structure for loading strategy
parameters from JSON files. Parameters are stored in a `HashMap<String, f64>`
for maximum flexibility.

## A/B Testing

`ab_testing::ab_test` runs two strategies sequentially using the existing
backtesting infrastructure and returns a pair of summaries for comparison.

## Examples

See `examples/sma_crossover.rs` for a very small demonstration strategy using
these utilities.
