# Jackbot Development Environment Setup

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

## Contributing

Please read our contribution guidelines before submitting pull requests.

## License

[Include your license information here] 