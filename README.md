# Jackbot Development Environment Setup

Jackbot Terminal is the fastest, battle tested power horse in the same cloud region of exchanges for minimum latency. Written in Rust for security and performance. Institutional grade trading meets great engineering.

This repository contains scripts to set up the development environment for the Jackbot project on Ubuntu.

## Getting Started

### Prerequisites

- Ubuntu Linux (tested on Ubuntu 20.04 LTS and newer)
- Sudo access

### Installation

1. Clone this repository:
   ```bash
   git clone https://github.com/yourusername/jackbot-sensor.git
   cd jackbot-sensor
   ```

2. Make the setup script executable:
   ```bash
   chmod +x setup_dev_environment.sh
   ```

3. Run the setup script:
   ```bash
   ./setup_dev_environment.sh
   ```

4. After installation completes, you may need to restart your terminal or run:
   ```bash
   source $HOME/.cargo/env
   ```

### What Gets Installed

The setup script installs:

- **Rust and Cargo**: The programming language and package manager used by the project
- **Redis**: For caching order book data and real-time information
- Writes use Redis pipelines with `.atomic()` to ensure consistency
- **AWS CLI**: For S3 interactions to store data
- **Python**: For supporting tools and ML components
- **Development libraries**: Required for building Rust dependencies
- **Apache Arrow dependencies**: For Parquet file format support

## Post-Installation

After installing the dependencies:

1. Configure your exchange API keys (see documentation)
2. Build the project: `cargo build`
3. Run tests to verify everything works: `cargo test`

## Project Structure

For details on the project structure and implementation status, see [IMPLEMENTATION_STATUS.md](docs/IMPLEMENTATION_STATUS.md).
Documentation for the API rate limiter lives in [RATE_LIMITING.md](docs/RATE_LIMITING.md).
## Feature Matrix (Summary)

A detailed feature matrix is maintained in [docs/IMPLEMENTATION_STATUS.md](docs/IMPLEMENTATION_STATUS.md). At a glance:

- **L2 Order Books**
  - Completed: Binance (Spot & Futures), Coinbase (Spot), Kraken (Spot & Futures), Bybit (Spot & Futures)
  - Partial: OKX (Spot & Futures), Kucoin (Spot)
  - Pending: Kucoin (Futures), Hyperliquid, Bitget
  - New: MEXC (Spot & Futures), Gate.io, and Crypto.com with Redis snapshot integration
- **Canonical Order Book** implemented across Binance, Bybit, OKX, Coinbase, and Kraken (Futures).
- **Trade Streams & Execution**: planned and under active development.

## TWAP Execution

`jackbot-execution` now includes a `twap` module capable of slicing large orders
into randomized chunks and scheduling them based on order book analytics from
`jackbot-data`. This enables discrete time-weighted execution both in
simulation with the `MockExchange` and against real venues.

## VWAP Execution

`jackbot-execution` also provides a `vwap` module for volume-weighted execution.
Orders can be split according to observed volume patterns and dispatched using
order book analytics, allowing more discrete participation in the market.


## Contributing

Please read our contribution guidelines before submitting pull requests.

## License

[Include your license information here] 
## Snapshotting Redis Data to S3

The `jackbot-snapshot` crate demonstrates extracting multi-exchange order book and trade data from Redis, serializing it to Parquet, uploading snapshots to a partitioned S3 layout, and tracking them with a minimal Iceberg table. A configurable `SnapshotScheduler` handles periodic persistence and retention cleanup.

